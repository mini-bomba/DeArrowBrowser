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

use std::{collections::VecDeque, ops::Deref, sync::LazyLock};

use actix_web::{get, web, HttpResponse};
use error_handling::{anyhow, bail, ErrorContext, ResContext};
use dearrow_browser_api::sync::{InnertubeChannel, InnertubeVideo};
use regex::Regex;
use reqwest::Client;

use crate::{middleware::ETagCache, routes::DB_READ_ERR, state::{AppConfig, DBLock}, utils};

type JsonResult<T> = utils::Result<web::Json<T>>;

static IT_PLAYER_URL: LazyLock<reqwest::Url>  = LazyLock::new(|| reqwest::Url::parse("https://www.youtube.com/youtubei/v1/player").expect("Should be able to parse the IT_PLAYER_URL"));
static IT_BROWSE_URL: LazyLock<reqwest::Url>  = LazyLock::new(|| reqwest::Url::parse("https://www.youtube.com/youtubei/v1/browse").expect("Should be able to parse the IT_BROWSE_URL"));
static YT_BASE_URL: LazyLock<reqwest::Url>    = LazyLock::new(|| reqwest::Url::parse("https://www.youtube.com/").expect("Should be able to parse the YT_BASE_URL"));
// https://stackoverflow.com/a/16326307
static UCID_EXTRACTION_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"externalId":"([^"]+)""#).expect("Should be able to parse the UCID extraction regex"));
// https://github.com/yt-dlp/yt-dlp/blob/a065086640e888e8d58c615d52ed2f4f4e4c9d18/yt_dlp/extractor/youtube.py#L518-L519
pub static UCID_REGEX: LazyLock<Regex>        = LazyLock::new(|| Regex::new(r"^UC(?-u:[\w-]){22}$").expect("Should be able to parse the UCID regex"));
pub static HANDLE_REGEX: LazyLock<Regex>          = LazyLock::new(|| Regex::new(r"^@[\w.-]{3,30}$").expect("Should be able to parse the @handle regex"));

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
async fn get_innertube_video(path: web::Path<String>, client: web::Data<Client>, config: web::Data<AppConfig>) -> JsonResult<InnertubeVideo> {
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
async fn get_channel_endpoint(path: web::Path<String>, db_lock: DBLock) -> JsonResult<InnertubeChannel> {
    let channel_cache = {
        let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
        db.channel_cache.clone()
    };
    let channel_data = channel_cache.get_channel(path.into_inner().as_str()).await.context("Failed to get channel info")?;

    Ok(web::Json(InnertubeChannel {
        channel_name: channel_data.channel_name.deref().into(),
        total_videos: channel_data.total_videos as u64,
    }))
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


pub struct ChannelData {
    pub name: String,
    pub video_ids: Vec<String>,
}

pub async fn get_channel(client: &Client, ucid: &str) -> Result<ChannelData, ErrorContext> {
    if !UCID_REGEX.is_match(ucid) {
        bail!("Invalid UCID");
    }

    let mut channel_name: Option<String> = None;
    let mut video_ids = vec![];
    let mut pending_requests: VecDeque<it::browse::Input> = VecDeque::from([it::browse::Input {
        browse_id: Some(ucid),
        params: Some("EgZ2aWRlb3PyBgQKAjoA"),
        context: it::Context::default(),
        continuation: None,
    }]);

    while let Some(request) = pending_requests.pop_front() {
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

            resp.contents.two_column_browse_results_renderer.tabs.pop().context("Failed to decode browse channel response - decoded tabs list was empty")?
                .tab_renderer.content.rich_grid_renderer.contents
        };

        for res in results {
            match res {
                it::browse::out::RichGridItem::RichItemRenderer { content } => video_ids.push(content.video_renderer.video_id),
                it::browse::out::RichGridItem::ContinuationItemRenderer { continuation_endpoint } => pending_requests.push_back(it::browse::Input {
                    continuation: Some(continuation_endpoint.continuation_command.token),
                    context: it::Context::default(),
                    browse_id: None,
                    params: None,
                }),
            }
        }
    }

    Ok(ChannelData {
        name: channel_name.expect("Channel name should've been found"),
        video_ids,
    })
}


mod it {
    use serde::Serialize;
    use serde_with::skip_serializing_none;

    // https://github.com/ajayyy/DeArrow/blob/c4e1375380bc3b0cb202af283f0e7b4e5e6e30f1/src/thumbnails/thumbnailData.ts#L233
    #[derive(Serialize, Clone, Default)]
    pub struct Context<'a> {
        pub client: Client<'a>,
    }

    // https://github.com/ajayyy/DeArrow/blob/c4e1375380bc3b0cb202af283f0e7b4e5e6e30f1/src/thumbnails/thumbnailData.ts#L234
    #[skip_serializing_none]
    #[derive(Serialize, Clone)]
    pub struct Client<'a> {
        #[serde(rename="clientName")]
        pub client_name: &'static str,
        #[serde(rename="clientVersion")]
        pub client_version: &'static str,
        #[serde(rename="visitorData")]
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
        pub struct Input<'a> {
            pub context: super::Context<'a>,
            #[serde(rename="videoId")]
            pub video_id: &'a str,
            #[serde(rename="serviceIntegrityDimensions")]
            pub service_integrity_dimensions: Option<Sid<'a>>,
        }

        #[derive(Serialize, Clone)]
        pub struct Sid<'a> {
            #[serde(rename="poToken")]
            pub po_token: &'a str
        }

        pub mod out {
            use serde::Deserialize;
            use serde_with::{serde_as, DisplayFromStr};

            #[derive(Deserialize)]
            pub struct Video {
                #[serde(rename="videoDetails")]
                pub video_details: VideoDetails,
            }

            #[serde_as]
            #[derive(Deserialize)]
            pub struct VideoDetails {
                #[serde(rename="videoId")]
                pub video_id: String,
                #[serde(rename="lengthSeconds")]
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
        pub struct Input<'a> {
            pub context: super::Context<'a>,
            #[serde(rename="browseId")]
            pub browse_id: Option<&'a str>,
            pub continuation: Option<String>,
            pub params: Option<&'a str>,
        }

        pub mod out {
            use serde::Deserialize;
            use serde_with::{serde_as, VecSkipError};


            #[derive(Deserialize, Clone)]
            pub struct Channel {
                pub contents: ChannelContents,
                pub microformat: Microformat,
            }

            #[derive(Deserialize, Clone)]
            pub struct Microformat {
                #[serde(rename="microformatDataRenderer")]
                pub microformat_data_renderer: MicroformatDataRenderer,
            }

            #[derive(Deserialize, Clone)]
            pub struct MicroformatDataRenderer {
                pub title: String,
            }

            #[derive(Deserialize, Clone)]
            pub struct ChannelContents {
                #[serde(rename="twoColumnBrowseResultsRenderer")]
                pub two_column_browse_results_renderer: BrowseResultsRenderer,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            pub struct BrowseResultsRenderer {
                #[serde_as(as="VecSkipError<_>")]
                pub tabs: Vec<Tab>,
            }

            #[derive(Deserialize, Clone)]
            pub struct Tab {
                #[serde(rename="tabRenderer")]
                pub tab_renderer: TabRenderer,
            }

            #[derive(Deserialize, Clone)]
            pub struct TabRenderer {
                pub content: TabContent,
            }

            #[derive(Deserialize, Clone)]
            pub struct TabContent {
                #[serde(rename="richGridRenderer")]
                pub rich_grid_renderer: RichGridRenderer,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            pub struct RichGridRenderer {
                #[serde_as(as="VecSkipError<_>")]
                pub contents: Vec<RichGridItem>,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            pub struct Continuation {
                #[serde_as(as="VecSkipError<_>")]
                #[serde(rename="onResponseReceivedActions")]
                pub on_response_received_actions: Vec<ContinuationAction>,
            }

            #[derive(Deserialize, Clone)]
            pub struct ContinuationAction {
                #[serde(rename="appendContinuationItemsAction")]
                pub append_continuation_items_action: AppendItemsAction,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            pub struct AppendItemsAction {
                #[serde_as(as="VecSkipError<_>")]
                #[serde(rename="continuationItems")]
                pub continuation_items: Vec<RichGridItem>
            }


            #[derive(Deserialize, Clone)]
            pub enum RichGridItem {
                #[serde(rename="richItemRenderer")]
                RichItemRenderer {
                    content: RichItemContent,
                },
                #[serde(rename="continuationItemRenderer")]
                ContinuationItemRenderer {
                    #[serde(rename="continuationEndpoint")]
                    continuation_endpoint: ContinuationEndpoint,
                },
            }

            #[derive(Deserialize, Clone)]
            pub struct ContinuationEndpoint {
                #[serde(rename="continuationCommand")]
                pub continuation_command: ContinuationCommand,
            }

            #[derive(Deserialize, Clone)]
            pub struct ContinuationCommand {
                pub token: String,
            }

            #[derive(Deserialize, Clone)]
            pub struct RichItemContent {
                #[serde(rename="videoRenderer")]
                pub video_renderer: VideoRenderer,
            }

            #[derive(Deserialize, Clone)]
            pub struct VideoRenderer {
                #[serde(rename="videoId")]
                pub video_id: String,
            }
            
        }
    }
}
