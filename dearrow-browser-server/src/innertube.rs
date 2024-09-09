/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024 mini_bomba
*  
*  This program is free software: you can redistribute it and/or modify
*  it under the terms of the GNU Affero General Public License as published by
*  the Free Software Foundation, either version 3 of the License, or
*  (at your option) any later version.
*
*  This program is distributed in the hope that it will be useful,
*  but WITHOUT ANY WARRANTY; without even the implied warranty of
*  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*  GNU Affero General Public License for more details.
*
*  You should have received a copy of the GNU Affero General Public License
*  along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::{collections::{HashSet, VecDeque}, ops::Deref, sync::{atomic::Ordering, Arc}};

use actix_web::{get, web, Either, HttpResponse, http::StatusCode};
use error_handling::{anyhow, bail, ErrorContext, ResContext};
use dearrow_browser_api::sync::{InnertubeChannel, InnertubeVideo, self as api};
use log::warn;
use reqwest::Client;
use tokio::{fs::{remove_file, File}, io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter}};
use tokio_stream::{wrappers::LinesStream, StreamExt};

use crate::constants::*;
use crate::middleware::{ETagCache, ETagCacheControl};
use crate::state::{self, AppConfig, DBLock, GetChannelOutput};
use crate::utils::{self, link_file, ExtendResponder, ResponderExt};

type JsonResult<T> = utils::Result<web::Json<T>>;
type JsonResultOrFetchProgress<T> = utils::Result<Either<web::Json<T>, (ExtendResponder<web::Json<api::ChannelFetchProgress>>, StatusCode)>>;

pub fn configure_disabled(cfg: &mut web::ServiceConfig) {
    cfg.default_service(web::to(disabled_route));
}

pub fn configure_enabled(cfg: &mut web::ServiceConfig) {
    cfg.service(get_innertube_video)
       .service(get_channel_endpoint);
}

async fn disabled_route() -> HttpResponse {
    HttpResponse::NotFound().body("Innertube endpoints are disabled on this DeArrow Browser instance.")
}


// https://github.com/ajayyy/DeArrow/blob/c4e1375380bc3b0cb202af283f0e7b4e5e6e30f1/src/thumbnails/thumbnailData.ts#L230
#[get("/video/{video_id}")]
async fn get_innertube_video(path: web::Path<String>, client: web::ThinData<Client>, config: web::Data<AppConfig>) -> JsonResult<InnertubeVideo> {
    let vid = path.as_str();
    let url = IT_PLAYER_URL.clone();
    let input = {
        let mut context = it::Context::default();
        if let Some(ref visitor_data) = config.innertube.visitor_data {
            context.client.visitor_data = Some(visitor_data);
        }
        let sid = config.innertube.po_token.as_ref().map(|po_token| it::player::Sid { po_token });
        it::player::Input {
            context,
            video_id: vid,
            service_integrity_dimensions: sid,
        }
    };
    let mut req = client.post(url).json(&input);
    if let Some(ref visitor_data) = config.innertube.visitor_data {
        req = req.header("X-Goog-Visitor-Id", visitor_data);
    }
    let resp = req.send().await.context("Failed to send innertube request")?;
    let resp = resp.error_for_status().context("Innertube request failed")?;
    let result: it::player::out::Video = resp.json().await.context("Failed to deserialize innertube response")?;
    if result.video_details.video_id != vid {
        return Err(anyhow!("Innertube returned the wrong videoid - requested: {vid}, got: {}", result.video_details.video_id).into());
    }
    Ok(web::Json(InnertubeVideo {
        video_id: vid.into(),
        duration: result.video_details.length_seconds,
    }))
}

#[get("/channel/{handle}", wrap="ETagCache")]
async fn get_channel_endpoint(path: web::Path<String>, db_lock: DBLock) -> JsonResultOrFetchProgress<InnertubeChannel> {
    let channel_cache = {
        let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
        db.channel_cache.clone()
    };
    let channel_data = channel_cache.get_channel(path.into_inner().as_str()).await.context("Failed to get channel info")?;

    match channel_data {
        GetChannelOutput::Pending(progress) => {
            let mut resp = web::Json(api::ChannelFetchProgress::from(&progress)).extend();
            resp.extensions.insert(ETagCacheControl::DoNotCache);
            Ok(Either::Right((resp, *NOT_READY_YET)))
        },
        GetChannelOutput::Resolved(result) => {
            Ok(Either::Left(web::Json(InnertubeChannel {
                channel_name: result.channel_name.deref().into(),
                num_videos: result.num_videos as u64,
                num_vods: result.num_vods as u64,
                num_shorts: result.num_shorts as u64,
                total_videos: result.total_videos as u64,
            })))
        }
    }
}

pub async fn handle_to_ucid(client: &Client, handle: &str) -> Result<String, ErrorContext> {
    if UCID_REGEX.is_match(handle) {
        return Ok(handle.to_owned());
    }

    let url = if HANDLE_REGEX.is_match(handle) {
        let mut url = YT_BASE_URL.clone();
        url.path_segments_mut().expect("YT_BASE_URL should be a base or smth")
            .push(handle);
        url
    } else {
        let handle = format!("@{handle}");
        if HANDLE_REGEX.is_match(&handle) {
            let mut url = YT_BASE_URL.clone();
            url.path_segments_mut().expect("YT_BASE_URL should be a base or smth")
                .push(&handle);
            url
        } else {
            bail!("Invalid handle!");
        }
    };

    let resp = client.get(url).send().await.context("Failed to send channel page request")?;
    let resp = resp.error_for_status().context("Channel page request failed")?;
    let page = resp.text().await.context("Failed to receive the channel page")?;

    let Some(captures) = UCID_EXTRACTION_REGEX.captures(&page) else {
        bail!("Failed to find the UCID for this channel");
    };

    Ok(captures[1].to_owned())
}


pub struct BrowseChannelData {
    pub name: String,
    pub video_ids: Vec<String>,
}

pub struct BrowseMode {
    pub param: &'static str,
    pub cache_dir: &'static str,
}

pub async fn browse_channel(client: Client, config: Arc<AppConfig>, mode: &BrowseMode, ucid: Arc<str>, progress: Arc<state::BrowseProgress>) -> Result<BrowseChannelData, ErrorContext> {
    if !UCID_REGEX.is_match(&ucid) {
        bail!("Invalid UCID");
    }
    
    // Check fscache
    let fscache_dir = config.channel_cache_path.join(mode.cache_dir);
    let fscache_path = fscache_dir.join(&*ucid);
    let cached_video_ids: Vec<String> = match File::open(&fscache_path).await {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => vec![],
        Err(err) => {
            warn!("Got an unexpected error while trying to open the videos cache entry for channel UCID '{ucid}' for reading: {err}");
            vec![]
        },
        Ok(file) => {
            let file = BufReader::new(file);
            match LinesStream::new(file.lines()).filter(|r| !r.as_ref().is_ok_and(String::is_empty)).collect().await {
                Ok(v) => v,
                Err(err) => {
                    warn!("Got an unexpected error while trying to read the videos cache entry for channel UCID '{ucid}': {err}");
                    vec![]
                }
            }
        }
    };
    progress.videos_in_fscache.store(cached_video_ids.len(), Ordering::Relaxed);

    // Put into hashmap for easy searching
    let cached_video_ids_set: HashSet<&str> = cached_video_ids.iter().map(AsRef::as_ref).collect();

    // Fetch new vids via innertube
    let mut channel_name: Option<String> = None;
    let mut new_video_ids = vec![];
    let mut pending_requests: VecDeque<it::browse::Input> = VecDeque::from([it::browse::Input {
        browse_id: Some(&ucid),
        params: Some(mode.param),
        context: it::Context::default(),
        continuation: None,
    }]);

    'outer: while let Some(request) = pending_requests.pop_front() {
        let is_continuation = request.continuation.is_some();
        let resp = client.post(IT_BROWSE_URL.clone()).json(&request).send().await.context("Failed to send browse request")?;
        let resp = resp.error_for_status().context("Browse request failed")?;

        let results = if is_continuation {
            let mut resp: it::browse::out::Continuation = resp.json().await.context("Failed to decode browse continuation response")?;
            
            resp.on_response_received_actions.pop().context("Failed to decode browse continuation response - decoded actions list was empty")?
                .append_continuation_items_action.continuation_items
        } else {
            let mut resp: it::browse::out::Channel = resp.json().await.context("Failed to decode browse channel response")?;

            channel_name = Some(resp.microformat.microformat_data_renderer.title);

            let Some(tab) = resp.contents.two_column_browse_results_renderer.tabs.pop() else {
                if mode.param == IT_BROWSE_VIDEOS.param {
                    bail!("Failed to decode browse channel response - decoded tabs list was empty");
                }
                break 'outer;  // acceptable for other tabs
            };
            tab.tab_renderer.content.rich_grid_renderer.contents
        };

        for res in results {
            match res {
                it::browse::out::RichGridItem::RichItemRenderer { content } => {
                    let video_id = match content {
                        it::browse::out::RichItemContent::ReelItemRenderer { video_id } |
                        it::browse::out::RichItemContent::VideoRenderer { video_id } => video_id,
                    };
                    if cached_video_ids_set.contains(&*video_id) {
                        progress.videos_fetched.store(new_video_ids.len(), Ordering::Relaxed);
                        break 'outer;
                    }
                    new_video_ids.push(video_id);
                },
                it::browse::out::RichGridItem::ContinuationItemRenderer { continuation_endpoint } => pending_requests.push_back(it::browse::Input {
                    continuation: Some(continuation_endpoint.continuation_command.token),
                    context: it::Context::default(),
                    browse_id: None,
                    params: None,
                }),
            }
        }
        progress.videos_fetched.store(new_video_ids.len(), Ordering::Relaxed);
    }
    drop(cached_video_ids_set); // no longer needed

    // Arrange the final list
    if new_video_ids.is_empty() {
        new_video_ids = cached_video_ids;
    } else {
        new_video_ids.extend_from_slice(&cached_video_ids);
        drop(cached_video_ids);

        // Cache videoid list
        // First create the file as a temporary, unlinked file. Then link it after writing is finished.
        //
        // This essentially makes writing the cache file an atomic operation - if writing fails, no
        // changes to the file system are made.
        // Worst that can happen is we unlink the existing file and are unable to link the new one in.
        match File::options().write(true).custom_flags(libc::O_TMPFILE).open(fscache_dir).await {
            Err(err) => {
                warn!("Got an unexpected error while trying to open the videos cache entry for channel UCID '{ucid}' for writing: {err}");
            },
            Ok(mut file) => {
                let result: std::io::Result<()> = async {
                    let mut file = BufWriter::new(&mut file);
                    for videoid in &new_video_ids {
                        file.write_all(videoid.as_bytes()).await?;
                        file.write_all(b"\n").await?;
                    }
                    file.flush().await?;
                    Ok(())
                }.await;
                if let Err(err) = result {
                    warn!("Got an unexpected error while trying to write the videos cache entry for channel UCID '{ucid}': {err}");
                } else {
                    // Writing finished, swap the files.
                    match remove_file(&fscache_path).await {
                        Ok(()) => (),
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (), // this is fine
                        Err(err) => warn!("Failed to unlink existing videos cache entry for channel UCID '{ucid}': {err}"),
                    }
                    if let Err(err) = link_file(&file, &fscache_path) {
                        warn!("Failed to link in the new videos cache entry for channel UCID '{ucid}': {err}");
                    }
                }
            }
        }
    }


    Ok(BrowseChannelData {
        name: channel_name.expect("Channel name should've been found"),
        video_ids: new_video_ids,
    })
}


mod it {
    use serde::Serialize;
    use serde_with::skip_serializing_none;

    // https://github.com/ajayyy/DeArrow/blob/c4e1375380bc3b0cb202af283f0e7b4e5e6e30f1/src/thumbnails/thumbnailData.ts#L233
    #[derive(Serialize, Clone, Default)]
    #[serde(rename_all="camelCase")]
    pub struct Context<'a> {
        pub client: Client<'a>,
    }

    // https://github.com/ajayyy/DeArrow/blob/c4e1375380bc3b0cb202af283f0e7b4e5e6e30f1/src/thumbnails/thumbnailData.ts#L234
    #[skip_serializing_none]
    #[derive(Serialize, Clone)]
    #[serde(rename_all="camelCase")]
    pub struct Client<'a> {
        pub client_name: &'static str,
        pub client_version: &'static str,
        pub visitor_data: Option<&'a str>,
    }

    impl<'a> Default for Client<'a> {
        fn default() -> Self {
            Self {
                client_name: "WEB",
                client_version: "2.20240808.00.00",
                visitor_data: None,
            }
        }
    }

    pub mod player {
        use serde::Serialize;
        use serde_with::skip_serializing_none;

        // https://github.com/ajayyy/DeArrow/blob/c4e1375380bc3b0cb202af283f0e7b4e5e6e30f1/src/thumbnails/thumbnailData.ts#L232
        #[skip_serializing_none]
        #[derive(Serialize, Clone)]
        #[serde(rename_all="camelCase")]
        pub struct Input<'a> {
            pub context: super::Context<'a>,
            pub video_id: &'a str,
            pub service_integrity_dimensions: Option<Sid<'a>>,
        }

        #[derive(Serialize, Clone)]
        #[serde(rename_all="camelCase")]
        pub struct Sid<'a> {
            pub po_token: &'a str
        }

        pub mod out {
            use serde::Deserialize;
            use serde_with::{serde_as, DisplayFromStr};

            #[derive(Deserialize)]
            #[serde(rename_all="camelCase")]
            pub struct Video {
                pub video_details: VideoDetails,
            }

            #[serde_as]
            #[derive(Deserialize)]
            #[serde(rename_all="camelCase")]
            pub struct VideoDetails {
                pub video_id: String,
                #[serde_as(as="DisplayFromStr")]
                pub length_seconds: u64,
            }
            
        }
        
    }

    pub mod browse {
        use serde::Serialize;
        use serde_with::skip_serializing_none;

        #[skip_serializing_none]
        #[derive(Serialize, Clone)]
        #[serde(rename_all="camelCase")]
        pub struct Input<'a> {
            pub context: super::Context<'a>,
            pub browse_id: Option<&'a str>,
            pub continuation: Option<String>,
            pub params: Option<&'a str>,
        }

        pub mod out {
            use serde::Deserialize;
            use serde_with::{serde_as, VecSkipError};


            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct Channel {
                pub contents: ChannelContents,
                pub microformat: Microformat,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct Microformat {
                pub microformat_data_renderer: MicroformatDataRenderer,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct MicroformatDataRenderer {
                pub title: String,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct ChannelContents {
                pub two_column_browse_results_renderer: BrowseResultsRenderer,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct BrowseResultsRenderer {
                #[serde_as(as="VecSkipError<_>")]
                pub tabs: Vec<Tab>,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct Tab {
                pub tab_renderer: TabRenderer,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct TabRenderer {
                pub content: TabContent,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct TabContent {
                pub rich_grid_renderer: RichGridRenderer,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct RichGridRenderer {
                #[serde_as(as="VecSkipError<_>")]
                pub contents: Vec<RichGridItem>,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct Continuation {
                #[serde_as(as="VecSkipError<_>")]
                pub on_response_received_actions: Vec<ContinuationAction>,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct ContinuationAction {
                pub append_continuation_items_action: AppendItemsAction,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct AppendItemsAction {
                #[serde_as(as="VecSkipError<_>")]
                pub continuation_items: Vec<RichGridItem>
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase", rename_all_fields="camelCase")]
            pub enum RichGridItem {
                RichItemRenderer {
                    content: RichItemContent,
                },
                ContinuationItemRenderer {
                    continuation_endpoint: ContinuationEndpoint,
                },
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct ContinuationEndpoint {
                pub continuation_command: ContinuationCommand,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct ContinuationCommand {
                pub token: String,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase", rename_all_fields="camelCase")]
            pub enum RichItemContent {
                VideoRenderer {
                    video_id: String,
                },
                ReelItemRenderer {
                    video_id: String,
                }
            }
            //
            // pub struct RichItemContent {
            //     #[serde(rename="videoRenderer")]
            //     pub video_renderer: VideoRenderer,
            // }
            //
            // #[derive(Deserialize, Clone)]
            // pub struct VideoRenderer {
            //     #[serde(rename="videoId")]
            //     pub video_id: String,
            // }
            
        }
    }
}
