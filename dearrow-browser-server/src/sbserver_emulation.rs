/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2024 mini_bomba
*
*  This file contains code adapted from the SponsorBlockServer project, licensed under AGPL 3.0
*  https://github.com/ajayyy/SponsorBlockServer
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
#![allow(clippy::needless_pass_by_value)]
use std::{sync::Arc, collections::HashMap};

use actix_web::{get, http::StatusCode, post, web, CustomizeResponder, HttpResponse, Responder};
use alea_js::Alea;
use error_handling::anyhow;
use dearrow_parser::{Extension, Thumbnail, ThumbnailFlags, Title, TitleFlags, VideoInfo};
use serde::{Deserialize, Serialize};

use crate::{middleware::ETagCache, state::{DBLock, StringSetLock}, utils};
use crate::constants::*;

type JsonResult<T> = utils::Result<web::Json<T>>;
type CustomizedJsonResult<T> = utils::Result<CustomizeResponder<web::Json<T>>>;

pub fn configure_disabled(cfg: &mut web::ServiceConfig) {
    cfg.default_service(web::to(disabled_route));
}

pub fn configure_enabled(cfg: &mut web::ServiceConfig) {
    cfg.service(get_video_branding)
        .service(post_video_branding)
        .service(get_chunk_branding)
        .service(get_user_info)
        .default_service(web::to(unknown_route));

}

async fn disabled_route() -> HttpResponse {
    HttpResponse::NotFound().body("SponsorBlockServer emulation is disabled on this DeArrow Browser instance.")
}

async fn unknown_route() -> HttpResponse {
    HttpResponse::NotFound().body("Unknown or unimplemented endpoint. Only /api/branding and /api/userInfo are implemented. See the README.md of DeArrow Browser to learn more about the limitations of SponsorBlockServer emulation.")
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct VideoBrandingParams {
    videoID: String,
    #[serde(default)]
    service: Option<String>,
    #[serde(default)]
    returnUserID: bool,
    #[serde(default)]
    fetchAll: bool,
}

#[allow(non_snake_case)]
#[derive(Serialize, Debug, Clone)]
struct SBApiTitle {
    title: String,
    original: bool,
    votes: i8,
    locked: bool,
    UUID: Arc<str>,
    #[serde(skip_serializing_if="Option::is_none")]
    userID: Option<Arc<str>>,
}

impl SBApiTitle {
    fn from_db(title: &Title, include_userid: bool) -> SBApiTitle {
        SBApiTitle {
            // https://github.com/ajayyy/SponsorBlockServer/blob/af31f511a53a7e30ad27123656a911393200672b/src/routes/getBranding.ts#L58
            title: title.title.replace('<', "‹"),
            original: title.flags.contains(TitleFlags::Original),
            votes: title.votes.saturating_sub(title.downvotes).saturating_sub(title.flags.contains(TitleFlags::Unverified).into()),
            locked: title.flags.contains(TitleFlags::Locked),
            UUID: title.uuid.clone(),
            userID: include_userid.then(|| title.user_id.clone()),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Debug, Clone)]
struct SBApiThumbnail {
    timestamp: Option<f64>,
    original: bool,
    votes: i8,
    locked: bool,
    UUID: Arc<str>,
    #[serde(skip_serializing_if="Option::is_none")]
    userID: Option<Arc<str>>,
}

impl SBApiThumbnail {
    fn from_db(thumb: &Thumbnail, include_userid: bool) -> SBApiThumbnail {
        SBApiThumbnail {
            timestamp: thumb.timestamp,
            original: thumb.flags.contains(ThumbnailFlags::Original),
            votes: thumb.votes.saturating_sub(thumb.downvotes),
            locked: thumb.flags.contains(ThumbnailFlags::Locked),
            UUID: thumb.uuid.clone(),
            userID: include_userid.then(|| thumb.user_id.clone()),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Debug, Clone)]
struct SBApiVideo {
    titles: Vec<SBApiTitle>,
    thumbnails: Vec<SBApiThumbnail>,
    randomTime: f64,
    videoDuration: Option<f64>,
}

// https://github.com/ajayyy/SponsorBlockServer/blob/af31f511a53a7e30ad27123656a911393200672b/src/routes/getBranding.ts#L233
pub fn get_random_time_for_video(video_id: &str, video_info: Option<&VideoInfo>) -> f64 {
    let random_time = Alea::new(video_id).random();

    if let Some(video_info) = video_info {
        let mut random_time = if !video_info.has_outro && random_time > 0.9 {
            random_time - 0.9
        } else {
            random_time
        };

        // Scale to the unmarked length of the video
        random_time *= video_info.uncut_segments.iter().map(|s| s.length).sum::<f64>();

        // Then map it to the unmarked segments
        for segment in &video_info.uncut_segments {
            if random_time <= segment.length {
                random_time += segment.offset;
                break;
            }
            random_time -= segment.length;
        };

        random_time
    } else if random_time > 0.9 {
        random_time - 0.9
    } else {
        random_time
    }
}

fn unknown_video(video_id: &str) -> SBApiVideo {
    SBApiVideo {
        titles: vec![],
        thumbnails: vec![],
        randomTime: get_random_time_for_video(video_id, None),
        videoDuration: None,
    }
}

#[post("/api/branding")]
async fn post_video_branding() -> HttpResponse {
    HttpResponse::NotFound().body("Voting through DeArrow Browser is not supported. See the README.md of DeArrow Browser to learn more about the limitations of SponsorBlockServer emulation.")
}

#[get("/api/branding", wrap = "ETagCache")]
async fn get_video_branding(db_lock: DBLock, string_set: StringSetLock, query: web::Query<VideoBrandingParams>) -> CustomizedJsonResult<SBApiVideo> {
    let video_id = string_set.read().map_err(|_| SS_READ_ERR.clone())?
        .set.get(query.0.videoID.as_str()).cloned();
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    if let Some(service) = query.0.service {
        if service != "YouTube" {
            return Ok(web::Json(unknown_video(&query.0.videoID)).customize().with_status(StatusCode::NOT_FOUND));
        }
    }
    match video_id {
        None => Ok(web::Json(unknown_video(&query.0.videoID)).customize().with_status(StatusCode::NOT_FOUND)),
        Some(video_id) => {
            let video_info = db.db.get_video_info(&video_id);
            Ok(web::Json(SBApiVideo {
                titles: {
                    let mut titles: Vec<SBApiTitle> = db.db.titles.iter()
                        .filter(|t| 
                            Arc::ptr_eq(&t.video_id, &video_id) 
                            && t.votes > -1 
                            && t.votes.saturating_sub(t.downvotes) > -2 
                            && !t.flags.intersects(TitleFlags::Removed | TitleFlags::ShadowHidden | TitleFlags::MissingVotes)
                            && (
                                query.0.fetchAll 
                                || t.flags.contains(TitleFlags::Locked) 
                                || t.votes.saturating_sub(t.downvotes) >= t.flags.contains(TitleFlags::Unverified).into()
                            )
                        )
                        .map(|t| SBApiTitle::from_db(t, query.0.returnUserID))
                        .collect();
                    titles.sort_unstable_by(|a, b| a.locked.cmp(&b.locked).then(a.votes.cmp(&b.votes)).reverse());
                    titles
                },
                thumbnails: {
                    let mut thumbs: Vec<SBApiThumbnail> = db.db.thumbnails.iter()
                        .filter(|t|
                            Arc::ptr_eq(&t.video_id, &video_id) 
                            && t.votes.saturating_sub(t.downvotes) > -2 
                            && !t.flags.intersects(ThumbnailFlags::Removed | ThumbnailFlags::ShadowHidden | ThumbnailFlags::MissingVotes | ThumbnailFlags::MissingTimestamp)
                            && (
                                (query.0.fetchAll && !t.flags.contains(ThumbnailFlags::Original))
                                || t.flags.contains(ThumbnailFlags::Locked)
                                || t.votes.saturating_sub(t.downvotes) >= t.flags.contains(ThumbnailFlags::Original).into()
                            )
                        )
                        .map(|t| SBApiThumbnail::from_db(t, query.0.returnUserID))
                        .collect();
                    thumbs.sort_unstable_by(|a, b| a.locked.cmp(&b.locked).then(a.votes.cmp(&b.votes).then(a.original.cmp(&b.original).reverse())).reverse());
                    thumbs
                },
                randomTime: get_random_time_for_video(&video_id, video_info),
                videoDuration: video_info.map(|v| v.video_duration),
            }).customize())
        }
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct ChunkBrandingParams {
    #[serde(default)]
    service: Option<String>,
    #[serde(default)]
    returnUserID: bool,
    #[serde(default)]
    fetchAll: bool,
}

#[derive(Deserialize, Debug)]
struct ChunkBrandingPath {
    hash_prefix: String,
}

#[get("/api/branding/{hash_prefix}", wrap = "ETagCache")]
async fn get_chunk_branding(db_lock: DBLock, query: web::Query<ChunkBrandingParams>, path: web::Path<ChunkBrandingPath>) -> CustomizedJsonResult<HashMap<Arc<str>, SBApiVideo>> {
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    if let Some(service) = query.0.service {
        if service != "YouTube" {
            return Ok(web::Json(HashMap::new()).customize().with_status(StatusCode::NOT_FOUND));
        }
    }
    // validate & parse hashprefix
    if path.hash_prefix.len() != 4 {
        return Err(utils::Error::from(anyhow!("Unsupported hashprefix! Only 4-character prefixes are supported by DeArrow Browser's SponsorBlockServer emulation, but your was {} chars long!", path.hash_prefix.len())).set_status(StatusCode::BAD_REQUEST));
    }
    let hash_prefix = u16::from_str_radix(&path.hash_prefix, 16).map_err(|_| utils::Error::from(anyhow!("Invalid hashprefix!")).set_status(StatusCode::BAD_REQUEST))?;

    // Find and group details
    let mut videos: HashMap<Arc<str>, Option<&VideoInfo>> = db.db.video_infos[hash_prefix as usize].iter().map(|v| (v.video_id.clone(), Some(v))).collect();
    let mut titles: HashMap<Arc<str>, Vec<SBApiTitle>> = HashMap::new();
    db.db.titles.iter()
        .filter(|t|
            t.hash_prefix == hash_prefix
            && t.votes > -1 
            && !t.flags.intersects(TitleFlags::Removed | TitleFlags::ShadowHidden | TitleFlags::MissingVotes)
            && t.votes.saturating_sub(t.downvotes) > -2 
            && (
                query.0.fetchAll 
                || t.flags.contains(TitleFlags::Locked) 
                || t.votes.saturating_sub(t.downvotes) >= t.flags.contains(TitleFlags::Unverified).into()
            )
        )
        .for_each(|t| match titles.get_mut(&t.video_id) {
            Some(v) => v.push(SBApiTitle::from_db(t, query.0.returnUserID)),
            None => {
                titles.insert(t.video_id.clone(), vec![SBApiTitle::from_db(t, query.0.returnUserID)]);
                videos.entry(t.video_id.clone()).or_default();
            },
        });
    let mut thumbnails: HashMap<Arc<str>, Vec<SBApiThumbnail>> = HashMap::new();
    db.db.thumbnails.iter()
        .filter(|t|
            t.hash_prefix == hash_prefix
            && !t.flags.intersects(ThumbnailFlags::Removed | ThumbnailFlags::ShadowHidden | ThumbnailFlags::MissingVotes | ThumbnailFlags::MissingTimestamp)
            && t.votes.saturating_sub(t.downvotes) > -2 
            && (
                (query.0.fetchAll && !t.flags.contains(ThumbnailFlags::Original))
                || t.flags.contains(ThumbnailFlags::Locked)
                || t.votes.saturating_sub(t.downvotes) >= t.flags.contains(ThumbnailFlags::Original).into()
            )
        )
        .for_each(|t| match thumbnails.get_mut(&t.video_id) {
            Some(v) => v.push(SBApiThumbnail::from_db(t, query.0.returnUserID)),
            None => {
                thumbnails.insert(t.video_id.clone(), vec![SBApiThumbnail::from_db(t, query.0.returnUserID)]);
                videos.entry(t.video_id.clone()).or_default();
            },
        });

    // Construct response
    Ok(web::Json(videos.into_iter().map(|(v, info)| (v.clone(), SBApiVideo {
        titles: titles.get(&v).cloned().unwrap_or_default(),
        thumbnails: thumbnails.get(&v).cloned().unwrap_or_default(),
        randomTime: get_random_time_for_video(&v, info),
        videoDuration: info.map(|info| info.video_duration),
    })).collect::<HashMap<Arc<str>, SBApiVideo>>()).customize())
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct UserInfoParams {
    publicUserID: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Debug)]
struct UserInfo {
    userID: Arc<str>,
    userName: Arc<str>,
    titleSubmissionCount: usize,
    thumbnailSubmissionCount: usize,
    vip: bool,
    warnings: usize,
    warningReason: Option<Arc<str>>,
    deArrowWarningReason: Option<Arc<str>>,
}

#[get("/api/userInfo", wrap = "ETagCache")]
async fn get_user_info(db_lock: DBLock, string_set: StringSetLock, query: web::Query<UserInfoParams>) -> JsonResult<UserInfo> {
    let user_id = string_set.read().map_err(|_| SS_READ_ERR.clone())?
        .set.get(query.0.publicUserID.as_str()).cloned();
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(match user_id {
        None => {
            let user_id: Arc<str> = query.0.publicUserID.into();
            UserInfo { 
                userID: user_id.clone(),
                userName: user_id,
                titleSubmissionCount: 0,
                thumbnailSubmissionCount: 0,
                vip: false,
                warnings: 0,
                warningReason: None,
                deArrowWarningReason: None,
            }
        },
        Some(user_id) => {
            let active_warnings: Vec<_> = db.db.warnings.iter().rev().filter(|w| w.active && Arc::ptr_eq(&w.warned_user_id, &user_id)).collect();
            UserInfo {
                userName: db.db.usernames.get(&user_id).map_or_else(|| user_id.clone(), |u| u.username.clone()),
                titleSubmissionCount: db.db.titles.iter().filter(|t| Arc::ptr_eq(&t.user_id, &user_id) && t.votes >= 0).count(),
                thumbnailSubmissionCount: db.db.thumbnails.iter().filter(|t| Arc::ptr_eq(&t.user_id, &user_id) && t.votes >= 0).count(),
                vip: db.db.vip_users.contains(&user_id),
                userID: user_id,
                warnings: active_warnings.iter().filter(|w| w.extension == Extension::SponsorBlock).count(),
                warningReason: active_warnings.iter().filter(|w| w.extension == Extension::SponsorBlock).map(|w| w.message.clone()).next(),
                deArrowWarningReason: active_warnings.iter().filter(|w| w.extension == Extension::DeArrow).map(|w| w.message.clone()).next(),
            }
        },
    }))
}
