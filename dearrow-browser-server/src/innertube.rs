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

use std::{collections::{HashSet, VecDeque}, ops::Deref, sync::{atomic::Ordering, Arc}, str::FromStr};

use actix_web::{get, http::StatusCode, web, Either, HttpResponse};
use error_handling::{anyhow, bail, ErrContext, ErrorContext, ResContext};
use dearrow_browser_api::sync::{InnertubeChannel, InnertubeVideo, self as api};
use log::{debug, warn};
use reqwest::Client;
use tokio::{fs::File, io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter}, task::JoinSet};
use tokio_stream::{wrappers::LinesStream, StreamExt};

use crate::{constants::*, utils::{ReqwestResponseExt, TemporaryFile}};
use crate::middleware::{ETagCache, ETagCacheControl};
use crate::state::{self, AppConfig, DBLock, GetChannelOutput};
use crate::utils::{self, ExtendResponder, ResponderExt};

type JsonResult<T> = utils::Result<web::Json<T>>;
type JsonResultOrFetchProgress<T> = utils::Result<Either<web::Json<T>, (ExtendResponder<web::Json<api::ChannelFetchProgress>>, StatusCode)>>;

pub fn configure_disabled(cfg: &mut web::ServiceConfig) {
    cfg.default_service(web::to(disabled_route));
}

pub fn configure_enabled(cfg: &mut web::ServiceConfig) {
    cfg.service(get_innertube_video)
       .service(get_channel_endpoint);
       // .service(get_playlist_endpoint);
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
                num_releases: result.num_releases as u64,
                total_videos: result.total_videos as u64,
            })))
        }
    }
}
//
// #[get("/channel_albums/{handle}")]
// async fn get_playlist_endpoint(path: web::Path<String>, client: web::ThinData<Client>, config: web::Data<AppConfig>) -> JsonResult<Vec<Vec<String>>> {
//     let ucid = handle_to_ucid(&client, path.into_inner().as_str()).await?;
//     Ok(web::Json(browse_releases_tab((*client).clone(), config.clone().into_inner(), ucid.into(), Arc::default()).await?))
// }

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

#[derive(Clone, Copy)]
pub struct BrowseMode {
    pub param: &'static str,
    pub cache_dir: &'static str,
    pub tab_name: &'static str,
}

fn parse_yt_number(num: &str) -> Option<usize> {
    let num = num.replace(',', "");
    let num = NUMBER_REGEX.find(&num)?.as_str();
    usize::from_str(num).ok()
}

pub async fn browse_channel(client: Client, config: Arc<AppConfig>, mode: &BrowseMode, ucid: Arc<str>, progress: Arc<state::BrowseProgress>) -> Result<BrowseChannelData, ErrorContext> {
    if !UCID_REGEX.is_match(&ucid) {
        bail!("Invalid UCID");
    }
    
    // Check fscache
    let fscache_path = {
        let mut path = config.cache_path.join(mode.cache_dir);
        path.push(&*ucid);
        path
    };
    let fscache_tmpdir = config.cache_path.join(FSCACHE_TEMPDIR);
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
            let mut resp: it::browse::out::RichGridContinuation = resp.json_debug(mode.tab_name).await.context("Failed to decode browse continuation response")?;
            
            resp.on_response_received_actions.pop().context("Failed to decode browse continuation response - decoded actions list was empty")?
                .append_continuation_items_action.continuation_items
        } else {
            let mut resp: it::browse::out::BrowseOutput = resp.json_debug(mode.tab_name).await.context("Failed to decode browse channel response")?;

            channel_name = Some(resp.microformat.microformat_data_renderer.title);

            let Some(tab) = resp.contents.two_column_browse_results_renderer.tabs.pop() else {
                debug!("{} browsing aborted: tab unavailable", mode.tab_name);
                break 'outer;  // tab not available
            };
            if !tab.tab_renderer.title.is_some_and(|s| s == mode.tab_name) {
                debug!("{} browsing aborted: incorrect tab", mode.tab_name);
                break 'outer;  // tab not available
            }
            #[allow(clippy::match_wildcard_for_single_variants)]
            match tab.tab_renderer.content {
                it::browse::out::TabContent::RichGridRenderer { contents } => contents,
                _ => bail!("Failed to decode browse channel response - wrong video renderer kind returned"),
            }
        };

        for res in results {
            match res {
                it::browse::out::RichGridItem::RichItemRenderer { content } => {
                    let video_id = match content {
                        it::browse::out::RichItemContent::ReelItemRenderer { video_id } |
                        it::browse::out::RichItemContent::VideoRenderer { video_id } => video_id,
                        it::browse::out::RichItemContent::ShortsLockupViewModel { on_tap } => on_tap.innertube_command.reel_watch_endpoint.video_id,
                        it::browse::out::RichItemContent::PlaylistRenderer { .. } => bail!("Found a playlist in a video grid"),
                    };
                    if cached_video_ids_set.contains(&*video_id) {
                        progress.videos_fetched.store(new_video_ids.len(), Ordering::Relaxed);
                        debug!("{} browsing ended early: encountered a video already in cache", mode.tab_name);
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
        match TemporaryFile::new(fscache_path, &fscache_tmpdir).await {
            Err(err) => {
                warn!("Got an unexpected error while trying to open the videos cache entry for channel UCID '{ucid}' for writing: {err}");
            },
            Ok(mut file) => {
                let result: std::io::Result<()> = async {
                    let mut file = BufWriter::new(&mut *file);
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
                    // Writing finished, commit changes.
                    if let Err(err) = file.commit().await {
                        warn!("Got an unexpected error while trying to commit the videos cache entry for channel UCID '{ucid}': {err:?}");
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

pub async fn browse_playlist(client: Client, config: Arc<AppConfig>, plid: String, count_hint: Option<usize>, progress: Arc<state::BrowseProgress>) -> Result<Vec<String>, ErrorContext> {
    let fscache_path = { 
        let mut path = config.cache_path.join(FSCACHE_PLAYLISTS);
        path.push(&*plid);
        path
    };
    let fscache_tmpdir = config.cache_path.join(FSCACHE_TEMPDIR);
    
    let cached_video_ids: Vec<String> = match File::open(&fscache_path).await {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => vec![],
        Err(err) => {
            warn!("Got an unexpected error while trying to open the playlist cache entry for '{plid}' for reading: {err}");
            vec![]
        },
        Ok(file) => {
            let file = BufReader::new(file);
            match LinesStream::new(file.lines()).filter(|r| !r.as_ref().is_ok_and(String::is_empty)).collect().await {
                Ok(v) => v,
                Err(err) => {
                    warn!("Got an unexpected error while trying to read the playlist cache entry for '{plid}': {err}");
                    vec![]
                }
            }
        }
    };

    if let Some(count_hint) = count_hint { 
        if !cached_video_ids.is_empty() && count_hint == cached_video_ids.len() {
            // playlist size unchanged, assume contents haven't changed 
            // (which isn't a great assumption tbh)
            progress.videos_in_fscache.fetch_add(cached_video_ids.len(), Ordering::Relaxed);
            debug!("playlist browsing aborted: actual length equal to cached");
            return Ok(cached_video_ids);
        }
    }

    let mut new_video_ids = vec![];
    let playlist_browse_id = format!("VL{plid}");
    let mut pending_requests: VecDeque<it::browse::Input> = VecDeque::from([it::browse::Input {
        browse_id: Some(&playlist_browse_id),
        params: None,
        context: it::Context::default(),
        continuation: None,
    }]);

    while let Some(request) = pending_requests.pop_front() {
        let is_continuation = request.continuation.is_some();
        let resp = client.post(IT_BROWSE_URL.clone()).json(&request).send().await.context("Failed to send browse request")?;
        let resp = resp.error_for_status().context("Browse request failed")?;

        let results = if is_continuation {
            let mut resp: it::browse::out::PlaylistContinuation = resp.json_debug("playlist").await.context("Failed to decode browse continuation response")?;
            
            resp.on_response_received_actions.pop().context("Failed to decode browse continuation response - decoded actions list was empty")?
                .append_continuation_items_action.continuation_items
        } else {
            let mut resp: it::browse::out::BrowseOutput = resp.json_debug("playlist").await.context("Failed to decode browse playlist response")?;

            if count_hint.is_none() && !cached_video_ids.is_empty() {
                if let it::browse::out::BrowseHeader::PlaylistHeaderRenderer { num_videos_text } = resp.header {
                    if let Some(num) = num_videos_text.runs.first() {
                        let num = num.text.replace(',', "");
                        if let Ok(num) = usize::from_str(&num) {
                            if num == cached_video_ids.len() {
                                // playlist size unchanged, assume contents haven't changed 
                                // (which isn't a great assumption tbh)
                                progress.videos_in_fscache.fetch_add(cached_video_ids.len(), Ordering::Relaxed);
                                debug!("playlist browsing aborted: actual length equal to cached");
                                return Ok(cached_video_ids);
                            }
                        }
                    }
                }
            }

            let Some(tab) = resp.contents.two_column_browse_results_renderer.tabs.pop() else {
                bail!("Failed to decode browse playlist response - decoded tabs list was empty");
            };
            #[allow(clippy::match_wildcard_for_single_variants)]
            match tab.tab_renderer.content {
                it::browse::out::TabContent::SectionListRenderer { mut contents } => 
                    match contents.pop().context("Failed to decode browse playlist response - no section renderer found")?
                                  .item_section_renderer.contents.pop().context("Failed to decode browse playlist response - no playlist renderer found")? {
                        it::browse::out::ItemSectionItem::PlaylistVideoListRenderer { contents } => contents,
                        _ => bail!("Failed to decode browse playlist response - found a renderer other than PlaylistVideoListRenderer"),
                    }
                _ => bail!("Failed to decode browse playlist response - wrong video renderer kind returned"),
            }
        };

        let previous_len = new_video_ids.len();
        for res in results {
            match res {
                it::browse::out::PlaylistRendererItem::PlaylistVideoRenderer { video_id } => new_video_ids.push(video_id),
                it::browse::out::PlaylistRendererItem::ContinuationItemRenderer { continuation_endpoint } => pending_requests.push_back(it::browse::Input {
                    continuation: Some(continuation_endpoint.continuation_command.token),
                    context: it::Context::default(),
                    browse_id: None,
                    params: None,
                }),
            }
        }
        progress.videos_fetched.fetch_add(new_video_ids.len() - previous_len, Ordering::Relaxed);
    }

    // cache results
    if !new_video_ids.is_empty() {
        match TemporaryFile::new(fscache_path, &fscache_tmpdir).await {
            Err(err) => {
                warn!("Got an unexpected error while trying to open the playlist cache entry for '{plid}' for writing: {err}");
            },
            Ok(mut file) => {
                let result: std::io::Result<()> = async {
                    let mut file = BufWriter::new(&mut *file);
                    for videoid in &new_video_ids {
                        file.write_all(videoid.as_bytes()).await?;
                        file.write_all(b"\n").await?;
                    }
                    file.flush().await?;
                    Ok(())
                }.await;
                if let Err(err) = result {
                    warn!("Got an unexpected error while trying to write the playlist cache entry for '{plid}': {err}");
                } else {
                    // Writing finished, commit changes.
                    if let Err(err) = file.commit().await {
                        warn!("Got an unexpected error while trying to commit the playlist cache entry for '{plid}': {err:?}");
                    }
                }
            }
        }
    }

    Ok(new_video_ids)
}

// this targets registered artist channels
pub async fn browse_releases_tab(client: Client, config: Arc<AppConfig>, ucid: Arc<str>, progress: Arc<state::BrowseProgress>) -> Result<Vec<Vec<String>>, ErrorContext> {
    if !UCID_REGEX.is_match(&ucid) {
        bail!("Invalid UCID");
    }
    
    let mut album_fetch_tasks: JoinSet<Result<Vec<String>, ErrorContext>> = JoinSet::new();
    let mut pending_requests: VecDeque<it::browse::Input> = VecDeque::from([it::browse::Input {
        browse_id: Some(&ucid),
        params: Some(IT_BROWSE_RELEASES.param),
        context: it::Context::default(),
        continuation: None,
    }]);

    'outer: while let Some(request) = pending_requests.pop_front() {
        let is_continuation = request.continuation.is_some();
        let resp = client.post(IT_BROWSE_URL.clone()).json(&request).send().await.context("Failed to send browse request")?;
        let resp = resp.error_for_status().context("Browse request failed")?;

        let results = if is_continuation {
            let mut resp: it::browse::out::RichGridContinuation = resp.json_debug("releases_tab").await.context("Failed to decode browse continuation response")?;
            
            resp.on_response_received_actions.pop().context("Failed to decode browse continuation response - decoded actions list was empty")?
                .append_continuation_items_action.continuation_items
        } else {
            let mut resp: it::browse::out::BrowseOutput = resp.json_debug("releases_tab").await.context("Failed to decode browse channel response")?;

            let Some(tab) = resp.contents.two_column_browse_results_renderer.tabs.pop() else {
                debug!("release tab browsing aborted: tab unavailable");
                break 'outer;  // tab not available
            };
            if !tab.tab_renderer.title.is_some_and(|s| s == IT_BROWSE_RELEASES.tab_name) {
                debug!("release tab browsing aborted: incorrect tab");
                break 'outer;  // tab not available
            }
            #[allow(clippy::match_wildcard_for_single_variants)]
            match tab.tab_renderer.content {
                it::browse::out::TabContent::RichGridRenderer { contents } => contents,
                _ => bail!("Failed to decode browse channel response - wrong video renderer kind returned"),
            }
        };

        for res in results {
            match res {
                it::browse::out::RichGridItem::RichItemRenderer { content: it::browse::out::RichItemContent::PlaylistRenderer { playlist_id, video_count } } => {
                    album_fetch_tasks.spawn(browse_playlist(client.clone(), config.clone(), playlist_id, parse_yt_number(&video_count), progress.clone()));
                },
                it::browse::out::RichGridItem::RichItemRenderer { .. } => bail!("Found a video in a playlist grid"),
                it::browse::out::RichGridItem::ContinuationItemRenderer { continuation_endpoint } => pending_requests.push_back(it::browse::Input {
                    continuation: Some(continuation_endpoint.continuation_command.token),
                    context: it::Context::default(),
                    browse_id: None,
                    params: None,
                }),
            }
        }
    }

    let mut albums: Vec<Vec<String>> = vec![];
    while let Some(res) = album_fetch_tasks.join_next().await {
        match res {
            Err(e) => {
                album_fetch_tasks.detach_all();
                return Err(e.context("An album fetch task has panicked"));
            },
            Ok(Err(e)) => {
                album_fetch_tasks.detach_all();
                return Err(e.context("Fetching one of the albums has failed"));
            },
            Ok(Ok(r)) => albums.push(r),
        }
    }

    Ok(albums)
}

// this targets autogenerated topic channels
pub async fn browse_releases_homepage(client: Client, config: Arc<AppConfig>, ucid: Arc<str>, progress: Arc<state::BrowseProgress>) -> Result<Vec<Vec<String>>, ErrorContext> {
    if !UCID_REGEX.is_match(&ucid) {
        bail!("Invalid UCID");
    }
    
    let mut album_fetch_tasks: JoinSet<Result<Vec<String>, ErrorContext>> = JoinSet::new();
    let mut pending_requests: VecDeque<it::browse::Input> = VecDeque::from([it::browse::Input {
        browse_id: Some(&ucid),
        params: None,
        context: it::Context::default(),
        continuation: None,
    }]);

    'outer: while let Some(request) = pending_requests.pop_front() {
        let is_continuation = request.continuation.is_some();
        let resp = client.post(IT_BROWSE_URL.clone()).json(&request).send().await.context("Failed to send browse request")?;
        let resp = resp.error_for_status().context("Browse request failed")?;

        let results = if is_continuation {
            let mut resp: it::browse::out::GridContinuation = resp.json_debug("releases_home").await.context("Failed to decode browse continuation response")?;
            
            resp.on_response_received_actions.pop().context("Failed to decode browse continuation response - decoded actions list was empty")?
                .append_continuation_items_action.continuation_items
        } else {
            let mut resp: it::browse::out::BrowseOutput = resp.json_debug("releases_home").await.context("Failed to decode browse channel response")?;

            let Some(tab) = resp.contents.two_column_browse_results_renderer.tabs.pop() else {
                debug!("release browsing via homepage aborted: tab unavailable");
                break 'outer;  // tab not available
            };
            if !tab.tab_renderer.title.is_some_and(|s| s == IT_BROWSE_HOME.tab_name) {
                debug!("release browsing via homepage aborted: incorrect tab");
                break 'outer;  // tab not available
            }
            #[allow(clippy::match_wildcard_for_single_variants)]
            let it::browse::out::TabContent::SectionListRenderer { mut contents } = tab.tab_renderer.content else {
                bail!("Failed to decode browse channel response - wrong renderer kind returned");
            };
            let mut item_section_renderer = match contents.pop() {
                None => return Ok(vec![]), // Channel empty
                Some(c) => c.item_section_renderer,
            };
            let mut endpoint = match item_section_renderer.contents.pop() {
                Some(it::browse::out::ItemSectionItem::ShelfRenderer { mut title, endpoint }) => {
                    if title.runs.pop().is_some_and(|t| t.text == IT_RELEASES_SHELF_NAME) {
                        endpoint
                    } else {
                        debug!("release browsing via homepage aborted: no releases shelf found");
                        return Ok(vec![]) // Not an autogenerated topic channel
                    }
                },
                None | Some(_) => return Ok(vec![]), // Not an autogenerated topic channel
            };
            let mut item_section_renderer = endpoint.show_engagement_panel_endpoint.engagement_panel.engagement_panel_section_list_renderer.content.section_list_renderer
                                                    .contents.pop().context("Failed to decode releases shelf - no item section renderer found")?.item_section_renderer;
            let Some(it::browse::out::ItemSectionItem::ContinuationItemRenderer { continuation_endpoint }) = item_section_renderer.contents.pop() else {
                bail!("Failed to decode releases shelf - no continuation item renderer found");
            };

            // send another request!!!
            let resp = client.post(IT_BROWSE_URL.clone()).json(&it::browse::Input {
                continuation: Some(continuation_endpoint.continuation_command.token),
                context: it::Context::default(),
                browse_id: None,
                params: None,
            }).send().await.context("Failed to send followup browse continuation request")?;
            let resp = resp.error_for_status().context("Followup browse continuation request failed")?;
            let mut resp: it::browse::out::ShelfContinuation = resp.json_debug("releases_home").await.context("Failed to decode followup browse continuation response")?;

            resp.on_response_received_actions.pop().context("Failed to decode followup browse continuation response - decoded actions list was empty")?
                .append_continuation_items_action.continuation_items.pop().context("Failed to decode followup browse continuation response - no grid renderer found")?
                .grid_renderer.items
        };

        for res in results {
            match res {
                it::browse::out::GridItem::GridPlaylistRenderer { playlist_id, mut video_count_text } => {
                    let video_count = video_count_text.runs.pop().and_then(|t| parse_yt_number(&t.text));
                    album_fetch_tasks.spawn(browse_playlist(client.clone(), config.clone(), playlist_id, video_count, progress.clone()));
                }
                it::browse::out::GridItem::ContinuationItemRenderer { continuation_endpoint } => pending_requests.push_back(it::browse::Input {
                    continuation: Some(continuation_endpoint.continuation_command.token),
                    context: it::Context::default(),
                    browse_id: None,
                    params: None,
                }),
            }
        }
    }

    let mut albums: Vec<Vec<String>> = vec![];
    while let Some(res) = album_fetch_tasks.join_next().await {
        match res {
            Err(e) => {
                album_fetch_tasks.detach_all();
                return Err(e.context("An album fetch task has panicked"));
            },
            Ok(Err(e)) => {
                album_fetch_tasks.detach_all();
                return Err(e.context("Fetching one of the albums has failed"));
            },
            Ok(Ok(r)) => albums.push(r),
        }
    }

    Ok(albums)
}

#[allow(clippy::enum_variant_names)]
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
            use serde_with::{serde_as, VecSkipError, DefaultOnError};

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct BrowseOutput {
                pub contents: BrowseContents,
                pub microformat: Microformat,
                #[serde_as(as="DefaultOnError<_>")]
                pub header: BrowseHeader,
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
            pub struct BrowseContents {
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
                #[serde(default)]
                pub title: Option<String>,
                pub content: TabContent,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase", rename_all_fields="camelCase")]
            pub enum TabContent {
                RichGridRenderer {
                    #[serde_as(as="VecSkipError<_>")]
                    contents: Vec<RichGridItem>,
                },
                SectionListRenderer {
                    #[serde_as(as="VecSkipError<_>")]
                    contents: Vec<SectionListItem>,
                }
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase", bound(deserialize = "T: Deserialize<'de>"))]
            pub struct Continuation<T> {
                #[serde_as(as="VecSkipError<_>")]
                #[serde(alias = "onResponseReceivedEndpoints")]
                pub on_response_received_actions: Vec<ContinuationAction<T>>,
            }
            pub type RichGridContinuation = Continuation<RichGridItem>;
            pub type PlaylistContinuation = Continuation<PlaylistRendererItem>;
            pub type ShelfContinuation    = Continuation<ShelfContinuationItem>;
            pub type GridContinuation     = Continuation<GridItem>;

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct ContinuationAction<T> {
                pub append_continuation_items_action: AppendItemsAction<T>,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase", bound(deserialize = "T: Deserialize<'de>"))]
            pub struct AppendItemsAction<T> {
                #[serde_as(as="VecSkipError<_>")]
                pub continuation_items: Vec<T>
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
                },
                ShortsLockupViewModel {
                    on_tap: ShortsOnTap,
                },
                PlaylistRenderer {
                    playlist_id: String,
                    video_count: String,
                }
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct ShortsOnTap {
                pub innertube_command: InnertubeCommand,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct InnertubeCommand {
                pub reel_watch_endpoint: ReelWatchEndpoint,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct ReelWatchEndpoint {
                pub video_id: String,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct SectionListItem {
                pub item_section_renderer: ItemSectionRenderer,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct ItemSectionRenderer {
                #[serde_as(as="VecSkipError<_>")]
                pub contents: Vec<ItemSectionItem>,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase", rename_all_fields="camelCase")]
            pub enum ItemSectionItem {
                PlaylistVideoListRenderer {
                    #[serde_as(as="VecSkipError<_>")]
                    contents: Vec<PlaylistRendererItem>,
                },
                ShelfRenderer {
                    title: TextRuns,
                    endpoint: ShelfEndpoint,
                },
                ContinuationItemRenderer {
                    continuation_endpoint: ContinuationEndpoint,
                },
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase", rename_all_fields="camelCase")]
            pub enum PlaylistRendererItem {
                PlaylistVideoRenderer {
                    video_id: String
                },
                ContinuationItemRenderer {
                    continuation_endpoint: ContinuationEndpoint,
                },
            }

            #[derive(Deserialize, Clone, Default)]
            #[serde(rename_all="camelCase", rename_all_fields="camelCase")]
            pub enum BrowseHeader {
                PlaylistHeaderRenderer {
                    num_videos_text: TextRuns,
                },
                #[serde(other)]
                #[default]
                Unknown,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct TextRuns {
                pub runs: Vec<TextRun>,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct TextRun {
                pub text: String,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct ShelfEndpoint {
                pub show_engagement_panel_endpoint: PanelEndpoint,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct PanelEndpoint {
                pub engagement_panel: Panel,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct Panel {
                pub engagement_panel_section_list_renderer: PanelSectionListRenderer,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct PanelSectionListRenderer {
                pub content: PanelSectionListRendererContent,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct PanelSectionListRendererContent {
                pub section_list_renderer: SectionListRenderer,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct SectionListRenderer {
                #[serde_as(as="VecSkipError<_>")]
                pub contents: Vec<SectionListItem>,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct ShelfContinuationItem {
                pub grid_renderer: GridRenderer,
            }

            #[serde_as]
            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase")]
            pub struct GridRenderer {
                #[serde_as(as="VecSkipError<_>")]
                pub items: Vec<GridItem>,
            }

            #[derive(Deserialize, Clone)]
            #[serde(rename_all="camelCase", rename_all_fields="camelCase")]
            pub enum GridItem {
                GridPlaylistRenderer {
                    playlist_id: String,
                    video_count_text: TextRuns,
                },
                ContinuationItemRenderer {
                    continuation_endpoint: ContinuationEndpoint,
                },
            }
        }
    }
}
