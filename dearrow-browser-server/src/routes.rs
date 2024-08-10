/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2024 mini_bomba
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
use std::{collections::HashSet, sync::Arc};
use actix_web::{Responder, get, post, web, http::StatusCode, HttpResponse, rt::task::spawn_blocking};
use anyhow::{anyhow, bail, Context};
use chrono::{Utc, DateTime};
use dearrow_parser::{DearrowDB, ThumbnailFlags, TitleFlags};
use dearrow_browser_api::sync::*;
use log::warn;
use serde::Deserialize;

use crate::{built_info, middleware::ETagCache, sbserver_emulation::get_random_time_for_video, state::*, utils};

pub const SS_READ_ERR:  &str = "Failed to acquire StringSet for reading";
pub const SS_WRITE_ERR: &str = "Failed to acquire StringSet for writing";
pub const DB_READ_ERR:  &str = "Failed to acquire DatabaseState for reading";
pub const DB_WRITE_ERR: &str = "Failed to acquire DatabaseState for writing";

pub fn configure(app_config: web::Data<AppConfig>) -> impl FnOnce(&mut web::ServiceConfig) {
    return move |cfg| {
        cfg.service(helo)
           .service(get_titles)
           .service(get_unverified_titles)
           .service(get_broken_titles)
           .service(get_title_by_uuid)
           .service(get_titles_by_video_id)
           .service(get_titles_by_user_id)
           .service(get_thumbnails)
           .service(get_broken_thumbnails)
           .service(get_thumbnail_by_uuid)
           .service(get_thumbnails_by_video_id)
           .service(get_thumbnails_by_user_id)
           .service(get_user_by_userid)
           .service(get_video)
           .service(get_status)
           .service(get_errors)
           .service(request_reload);

        if app_config.innertube.enable {
            cfg.service(get_titles_by_channel)
               .service(get_thumbnails_by_channel);
        } else {
            cfg.route("/titles/channel/{channel}", web::route().to(innertube_disabled))
               .route("/titles/channel/{channel}", web::route().to(innertube_disabled));
        }
    };
}

type JsonResult<T> = utils::Result<web::Json<T>>;

#[derive(Deserialize)]
#[serde(default)]
pub struct MainEndpointURLParams {
    pub offset: usize,
    pub count: usize,
}

impl Default for MainEndpointURLParams {
    fn default() -> Self {
        Self {
            offset: 0,
            count: 50,
        }
    }
}

async fn innertube_disabled() -> HttpResponse {
    HttpResponse::NotFound().body("This endpoint requires making requests to innertube, which is disabled on this DeArrow Browser instance.")
}

#[get("/")]
async fn helo() -> impl Responder {
    "hi"
}

#[get("/status")]
async fn get_status(db_lock: DBLock, string_set: StringSetLock, config: web::Data<AppConfig>) -> JsonResult<StatusResponse> {
    let strings = match string_set.try_read() {
        Err(_) => None,
        Ok(set) => Some(set.set.len()),
    };
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    Ok(web::Json(StatusResponse {
        last_updated: db.last_updated,
        last_modified: db.last_modified,
        updating_now: db.updating_now,
        titles: db.db.titles.len(),
        thumbnails: db.db.thumbnails.len(),
        vip_users: db.db.vip_users.len(),
        usernames: db.db.usernames.len(),
        errors: db.errors.len(),
        string_count: strings,
        video_infos: db.video_info_count(),
        uncut_segments: db.uncut_segment_count(),
        server_version: built_info::PKG_VERSION.into(),
        server_git_hash: built_info::GIT_COMMIT_HASH.map(std::convert::Into::into),
        server_git_dirty: built_info::GIT_DIRTY,
        server_build_timestamp: DateTime::parse_from_rfc2822(built_info::BUILT_TIME_UTC).ok().map(|t| t.timestamp()),
        server_startup_timestamp: config.startup_timestamp.timestamp(),
    }))
}

#[derive(Deserialize, Debug)]
struct Auth {
    auth: Option<String>
}

fn do_reload(db_lock: DBLock, string_set_lock: StringSetLock, config: web::Data<AppConfig>) -> anyhow::Result<()> {
    {
        let mut db_state = db_lock.write().map_err(|_| anyhow!(DB_WRITE_ERR))?;
        if db_state.updating_now {
            bail!("Already updating!");
        }
        db_state.updating_now = true;
    }
    warn!("Reload requested");
    let mut string_set_clone = string_set_lock.read().map_err(|_| anyhow!(SS_READ_ERR))?.clone();
    let (mut new_db, errors) = DearrowDB::load_dir(config.mirror_path.as_path(), &mut string_set_clone)?;
    new_db.sort();
    let last_updated = Utc::now().timestamp_millis();
    let last_modified = utils::get_mtime(&config.mirror_path.join("titles.csv"));
    {
        let mut string_set = string_set_lock.write().map_err(|_| anyhow!(SS_WRITE_ERR))?;
        let mut db_state = db_lock.write().map_err(|_| anyhow!(DB_WRITE_ERR))?;
        *string_set = string_set_clone;
        *db_state = DatabaseState {
            db: new_db,
            errors: errors.into(),
            last_updated,
            last_modified,
            updating_now: false,
            etag: None,
            channel_cache: db_state.channel_cache.reset(),
        };
        db_state.etag = Some(db_state.generate_etag());
        string_set.clean();
    }
    warn!("Reload finished");
    Ok(())
}

#[post("/reload")]
async fn request_reload(db_lock: DBLock, string_set_lock: StringSetLock, config: web::Data<AppConfig>, auth: web::Query<Auth>) -> HttpResponse {
    let provided_hash = match auth.auth.as_deref() {
        None => { return HttpResponse::NotFound().finish(); },
        Some(s) => utils::sha256(s),
    };
    let actual_hash = utils::sha256(config.auth_secret.as_str());

    if provided_hash != actual_hash {
        return HttpResponse::Forbidden().finish();
    }
    match spawn_blocking(move || do_reload(db_lock, string_set_lock, config)).await {
        Ok(..) => HttpResponse::Ok().body("Reload complete"),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e:?}")),
    }
}

#[get("/errors")]
async fn get_errors(db_lock: DBLock) -> JsonResult<ErrorList> {
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    Ok(web::Json(db.errors.iter().map(|e| format!("{e:?}")).collect()))
}

#[get("/titles", wrap = "ETagCache")]
async fn get_titles(db_lock: DBLock, query: web::Query<MainEndpointURLParams>) -> JsonResult<Vec<ApiTitle>> {
    if query.count > 1024 {
        return Err(
            utils::Error::from(anyhow!("Too many requested titles. You requested {} titles, but the configured max is 1024.", query.count))
                .set_status(StatusCode::BAD_REQUEST)
        );
    }
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    Ok(web::Json(
        db.db.titles.iter().rev().skip(query.offset).take(query.count)
            .map(|t| t.into_with_db(&db.db)).collect::<Vec<_>>()
    ))
}

#[get("/titles/unverified", wrap = "ETagCache")]
async fn get_unverified_titles(db_lock: DBLock) -> JsonResult<Vec<ApiTitle>> {
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    Ok(web::Json(
        db.db.titles.iter().rev()
            .filter(|t| t.flags.contains(TitleFlags::Unverified) && !t.flags.intersects(TitleFlags::Locked | TitleFlags::ShadowHidden | TitleFlags::Removed) && t.votes-t.downvotes > -1)
            .map(|t| t.into_with_db(&db.db)).collect::<Vec<_>>()
    ))
}

#[get("/titles/broken", wrap = "ETagCache")]
async fn get_broken_titles(db_lock: DBLock) -> JsonResult<Vec<ApiTitle>> {
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    Ok(web::Json(
        db.db.titles.iter().rev()
            .filter(|t| t.flags.contains(TitleFlags::MissingVotes))
            .map(|t| t.into_with_db(&db.db)).collect::<Vec<_>>()
    ))
}

#[get("/titles/uuid/{uuid}", wrap = "ETagCache")]
async fn get_title_by_uuid(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>) -> JsonResult<ApiTitle> {
    let Some(uuid) = string_set.read().map_err(|_| anyhow!(SS_READ_ERR))?
            .set.get(path.into_inner().as_str()).cloned() else {
        return Err(utils::Error::EmptyStatus(StatusCode::NOT_FOUND));
    };
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    Ok(web::Json(
        db.db.titles.iter().find(|t| Arc::ptr_eq(&t.uuid, &uuid))
            .map(|t| t.into_with_db(&db.db))
            .ok_or(utils::Error::EmptyStatus(StatusCode::NOT_FOUND))?
    ))
}

#[get("/titles/video_id/{video_id}", wrap = "ETagCache")]
async fn get_titles_by_video_id(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>) -> JsonResult<Vec<ApiTitle>> {
    let video_id = string_set.read().map_err(|_| anyhow!(SS_READ_ERR))?
        .set.get(path.into_inner().as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    let titles = match video_id {
        None => vec![],
        Some(id) => db.db.titles.iter().rev()
            .filter(|title| Arc::ptr_eq(&title.video_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    Ok(web::Json(titles))
}

#[get("/titles/user_id/{user_id}", wrap = "ETagCache")]
async fn get_titles_by_user_id(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>) -> JsonResult<Vec<ApiTitle>> {
    let user_id = string_set.read().map_err(|_| anyhow!(SS_READ_ERR))?
        .set.get(path.into_inner().as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    let titles = match user_id {
        None => vec![],
        Some(id) => db.db.titles.iter().rev()
            .filter(|title| Arc::ptr_eq(&title.user_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    Ok(web::Json(titles))
}

#[get("/titles/channel/{channel}", wrap = "ETagCache")]
async fn get_titles_by_channel(db_lock: DBLock, path: web::Path<String>) -> JsonResult<Vec<ApiTitle>> {
    let channel_cache = {
        let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
        db.channel_cache.clone()
    };
    let channel_data = channel_cache.get_channel(path.into_inner().as_str()).await.context("Failed to get channel info")?;

    // we only really need the string pointer's address to figure out if they're equal, thanks to
    // the `StringSet`
    let vid_set: HashSet<usize> = channel_data.video_ids.iter().map(utils::arc_addr).collect();
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    let titles = db.db.titles.iter().rev()
        .filter(|title| vid_set.contains(&utils::arc_addr(&title.video_id)))
        .map(|t| t.into_with_db(&db.db))
        .collect();
    Ok(web::Json(titles))
}

#[get("/thumbnails", wrap = "ETagCache")]
async fn get_thumbnails(db_lock: DBLock, query: web::Query<MainEndpointURLParams>) -> JsonResult<Vec<ApiThumbnail>> {
    if query.count > 1024 {
        return Err(
            utils::Error::from(anyhow!("Too many requested thumbnails. You requested {} thumbnails, but the configured max is 1024.", query.count))
                .set_status(StatusCode::BAD_REQUEST)
        );
    }
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    Ok(web::Json(
        db.db.thumbnails.iter().rev().skip(query.offset).take(query.count)
            .map(|t| t.into_with_db(&db.db)).collect::<Vec<_>>()
    ))
}

#[get("/thumbnails/broken", wrap = "ETagCache")]
async fn get_broken_thumbnails(db_lock: DBLock) -> JsonResult<Vec<ApiThumbnail>> {
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    Ok(web::Json(
        db.db.thumbnails.iter().rev()
            .filter(|t| t.flags.intersects(ThumbnailFlags::MissingVotes | ThumbnailFlags::MissingTimestamp))
            .map(|t| t.into_with_db(&db.db)).collect::<Vec<_>>()
    ))
}


#[get("/thumbnails/uuid/{uuid}", wrap = "ETagCache")]
async fn get_thumbnail_by_uuid(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>) -> JsonResult<ApiThumbnail> {
    let Some(uuid) = string_set.read().map_err(|_| anyhow!(SS_READ_ERR))?
            .set.get(path.into_inner().as_str()).cloned() else {
        return Err(utils::Error::EmptyStatus(StatusCode::NOT_FOUND));
    };
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    Ok(web::Json(
        db.db.thumbnails.iter().find(|t| Arc::ptr_eq(&t.uuid, &uuid))
            .map(|t| t.into_with_db(&db.db))
            .ok_or(utils::Error::EmptyStatus(StatusCode::NOT_FOUND))?
    ))
}

#[get("/thumbnails/video_id/{video_id}", wrap = "ETagCache")]
async fn get_thumbnails_by_video_id(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>) -> JsonResult<Vec<ApiThumbnail>> {
    let video_id = string_set.read().map_err(|_| anyhow!(SS_READ_ERR))?
        .set.get(path.into_inner().as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    let titles = match video_id {
        None => vec![],
        Some(id) => db.db.thumbnails.iter().rev()
            .filter(|thumb| Arc::ptr_eq(&thumb.video_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    Ok(web::Json(titles))
}

#[get("/thumbnails/user_id/{video_id}", wrap = "ETagCache")]
async fn get_thumbnails_by_user_id(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>) -> JsonResult<Vec<ApiThumbnail>> {
    let user_id = string_set.read().map_err(|_| anyhow!(SS_READ_ERR))?
        .set.get(path.into_inner().as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    let titles = match user_id {
        None => vec![],
        Some(id) => db.db.thumbnails.iter().rev()
            .filter(|thumb| Arc::ptr_eq(&thumb.user_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    Ok(web::Json(titles))
}

#[get("/thumbnails/channel/{channel}", wrap = "ETagCache")]
async fn get_thumbnails_by_channel(db_lock: DBLock, path: web::Path<String>) -> JsonResult<Vec<ApiThumbnail>> {
    let channel_cache = {
        let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
        db.channel_cache.clone()
    };
    let channel_data = channel_cache.get_channel(path.into_inner().as_str()).await.context("Failed to get channel info")?;

    // we only really need the string pointer's address to figure out if they're equal, thanks to
    // the `StringSet`
    let vid_set: HashSet<usize> = channel_data.video_ids.iter().map(utils::arc_addr).collect();
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    let thumbs = db.db.thumbnails.iter().rev()
        .filter(|thumbnail| vid_set.contains(&utils::arc_addr(&thumbnail.video_id)))
        .map(|t| t.into_with_db(&db.db))
        .collect();
    Ok(web::Json(thumbs))
}

#[get("/users/user_id/{user_id}", wrap = "ETagCache")]
async fn get_user_by_userid(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>) -> JsonResult<User> {
    let user_id = string_set.read().map_err(|_| anyhow!(SS_READ_ERR))?
        .set.get(path.as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
    Ok(web::Json(match user_id {
        None => User {
            user_id: path.into_inner().into(),
            username: None, 
            username_locked: false,
            vip: false, 
            title_count: 0,
            thumbnail_count: 0
        },
        Some(user_id) => {
            let username = db.db.usernames.get(&user_id);
            User {
                user_id: user_id.clone(),
                username: username.map(|u| u.username.clone()),
                username_locked: username.map_or(false, |u| u.locked),
                vip: db.db.vip_users.contains(&user_id),
                title_count: db.db.titles.iter().filter(|t| Arc::ptr_eq(&t.user_id, &user_id)).count() as u64,
                thumbnail_count: db.db.thumbnails.iter().filter(|t| Arc::ptr_eq(&t.user_id, &user_id)).count() as u64,
            }
        }
    }))
}


fn unknown_video(video_id: Arc<str>) -> Video {
    Video { 
        random_thumbnail: get_random_time_for_video(&video_id, None),
        video_id,
        duration: None,
        fraction_unmarked: 1.,
        has_outro: false,
    }
}

#[get("/videos/{video_id}", wrap = "ETagCache")]
async fn get_video(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>) -> JsonResult<Video> {
    let video_id = string_set.read().map_err(|_| anyhow!(SS_READ_ERR))?
        .set.get(path.as_str()).cloned();
    Ok(web::Json(match video_id {
        None => unknown_video(path.as_str().into()),
        Some(video_id) => {
            let db = db_lock.read().map_err(|_| anyhow!(DB_READ_ERR))?;
            let video_info = db.db.get_video_info(&video_id);
            match video_info {
                None => unknown_video(video_id),
                Some(video_info) => Video { 
                    random_thumbnail: get_random_time_for_video(&video_id, Some(video_info)),
                    video_id,
                    duration: Some(video_info.video_duration),
                    fraction_unmarked: video_info.uncut_segments.iter().map(|s| s.length).sum(),
                    has_outro: video_info.has_outro,
                }
            }
        },
    }))
}
