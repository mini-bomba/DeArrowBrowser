/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2025 mini_bomba
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
use actix_web::Either;
use actix_web::{
    get, http::StatusCode, post, rt::task::spawn_blocking, web, HttpResponse, Responder,
};
use chrono::Utc;
use dearrow_browser_api::sync::{self as api, *};
use dearrow_parser::{db::DearrowDB, types::{ThumbnailFlags, TitleFlags}};
use cloneable_errors::{
    anyhow, bail, ErrorContext, IntoErrorIterator, ResContext, SerializableError,
};
use futures::join;
use log::{info, warn};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, sync::Arc};

use crate::built_info;
use crate::constants::*;
use crate::errors::extensions;
use crate::middleware::etag::{ETagCache, ETagCacheControl};
use crate::sbserver_emulation::get_random_time_for_video;
use crate::state::*;
use crate::utils::{self, ExtendResponder, ResponderExt};
use crate::errors::{self, Error::EmptyStatus};

pub fn configure(app_config: web::Data<AppConfig>) -> impl FnOnce(&mut web::ServiceConfig) {
    move |cfg| {
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
            .service(get_casual_titles)
            .service(get_thumbnail_by_uuid)
            .service(get_casual_titles_by_video_id)
            .service(get_user_by_userid)
            .service(get_users_by_username)
            .service(get_warnings)
            .service(get_user_warnings)
            .service(get_issued_warnings)
            .service(get_video)
            .service(get_status)
            .service(get_errors)
            .service(request_reload);

        if app_config.innertube.enable {
            cfg.service(get_titles_by_channel)
                .service(get_thumbnails_by_channel)
                .service(get_casual_titles_by_channel);
        } else {
            cfg.route(
                "/titles/channel/{channel}",
                web::route().to(innertube_disabled),
            )
            .route(
                "/titles/channel/{channel}",
                web::route().to(innertube_disabled),
            );
        }
    }
}

type JsonResult<T> = errors::Result<web::Json<T>>;
type JsonResultOrFetchProgress<T> = errors::Result<
    Either<
        web::Json<T>,
        (
            ExtendResponder<web::Json<api::ChannelFetchProgress>>,
            StatusCode,
        ),
    >,
>;

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
async fn get_status(
    db_lock: DBLock,
    string_set: StringSetLock,
    config: web::Data<AppConfig>,
) -> JsonResult<StatusResponse> {
    let strings = match string_set.try_read() {
        Err(_) => None,
        Ok(set) => Some(set.set.len()),
    };
    let channel_cache = {
        let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
        db.channel_cache.clone()
    };
    let (cached_channels, fscached_channels) = join!(
        channel_cache.num_channels_cached(),
        channel_cache.num_channels_fscached()
    );
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(StatusResponse {
        // database stats
        titles: Some(db.db.titles.len()),
        thumbnails: Some(db.db.thumbnails.len()),
        casual_titles: Some(db.db.casual_titles.len()),
        vip_users: Some(db.db.vip_users.len()),
        usernames: Some(db.db.usernames.len()),
        warnings: Some(db.db.warnings.len()),
        // dab internal stats
        errors: Some(db.errors.len()),
        string_count: strings,
        video_infos: Some(db.video_info_count),
        uncut_segments: Some(db.uncut_segment_count),
        cached_channels: Some(cached_channels),
        fscached_channels: Some(fscached_channels),
        // general server build data
        server_version: Some(SERVER_VERSION.clone()),
        server_git_hash: SERVER_GIT_HASH.clone(),
        server_git_dirty: built_info::GIT_DIRTY,
        server_build_timestamp: *BUILD_TIMESTAMP,
        server_startup_timestamp: Some(config.startup_timestamp.timestamp()),
        server_brand: Some(SERVER_BRAND.clone()),
        server_url: Some(SERVER_URL.clone()),
        // stats for snapshot-based impls
        last_updated: Some(db.last_updated),
        last_modified: Some(db.last_modified),
        updating_now: db.updating_now,
    }))
}

#[derive(Deserialize, Debug)]
struct Auth {
    auth: Option<String>,
}

fn do_reload(
    db_lock: DBLock,
    string_set_lock: StringSetLock,
    config: web::Data<AppConfig>,
) -> Result<(), ErrorContext> {
    {
        let mut db_state = db_lock.write().map_err(|_| DB_WRITE_ERR.clone())?;
        if db_state.updating_now {
            bail!("Already updating!");
        }
        db_state.updating_now = true;
    }
    warn!("Reload requested");
    let mut string_set_clone = string_set_lock
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .clone();
    let (new_db, errors) =
        DearrowDB::load_dir(config.mirror_path.as_path(), &mut string_set_clone, !config.skip_unused_usernames)?;
    if new_db.usernames_skipped != 0 {
        info!("Skipped loading {} usernames", new_db.usernames_skipped);
    }
    let last_updated = Utc::now().timestamp_millis();
    let last_modified = utils::get_mtime(&config.mirror_path.join("titles.csv"));
    {
        let mut string_set = string_set_lock.write().map_err(|_| SS_WRITE_ERR.clone())?;
        let mut db_state = db_lock.write().map_err(|_| DB_WRITE_ERR.clone())?;
        *string_set = string_set_clone;
        *db_state = DatabaseState {
            db: new_db,
            errors: errors.into(),
            last_updated,
            last_modified,
            updating_now: false,
            etag: None,
            channel_cache: db_state.channel_cache.reset(),
            uncut_segment_count: 0,
            video_info_count: 0,
        };
        db_state.uncut_segment_count = db_state.calculate_uncut_segment_count();
        db_state.video_info_count = db_state.calculate_video_info_count();
        db_state.etag = Some(db_state.generate_etag());
        string_set.clean();
    }
    warn!("Reload finished");
    Ok(())
}

#[post("/reload")]
async fn request_reload(
    db_lock: DBLock,
    string_set_lock: StringSetLock,
    config: web::Data<AppConfig>,
    auth: web::Query<Auth>,
) -> errors::Result<&'static str> {
    let provided_hash = match auth.auth.as_deref() {
        None => {
            return Err(EmptyStatus(StatusCode::NOT_FOUND));
        }
        Some(s) => Sha256::digest(s),
    };
    let actual_hash = Sha256::digest(config.auth_secret.as_str());

    if provided_hash != actual_hash {
        return Err(
            anyhow!("forbidden", extend:
                extensions::status::FORBIDDEN.clone(),
                extensions::empty_body::INSTANCE.clone(),
                extensions::no_timings::INSTANCE.clone()
            ).into()
        );
    }
    spawn_blocking(move || do_reload(db_lock, string_set_lock, config))
        .await
        .context("Reload task panicked")?
        .context("Failed to reload")?;
    Ok("Reload complete")
}

#[get("/errors")]
async fn get_errors(db_lock: DBLock) -> JsonResult<Vec<SerializableError>> {
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(
        db.errors
            .iter()
            .map(IntoErrorIterator::serializable_copy)
            .collect(),
    ))
}

#[get("/titles", wrap = "ETagCache")]
async fn get_titles(
    db_lock: DBLock,
    query: web::Query<MainEndpointURLParams>,
) -> JsonResult<Vec<ApiTitle>> {
    if query.count > 1024 {
        return Err(
            anyhow!(
                ("Too many requested titles. You requested {} titles, but the configured max is 1024.", query.count),
                extend: extensions::status::BAD_REQUEST.clone()
            ).into()
        );
    }
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(
        db.db
            .titles
            .iter()
            .rev()
            .skip(query.offset)
            .take(query.count)
            .map(|t| t.into_with_db(&db.db))
            .collect::<Vec<_>>(),
    ))
}

#[get("/titles/unverified", wrap = "ETagCache")]
async fn get_unverified_titles(db_lock: DBLock) -> JsonResult<Vec<ApiTitle>> {
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(
        db.db
            .titles
            .iter()
            .rev()
            .filter(|t| {
                t.flags.contains(TitleFlags::Unverified)
                    && !t.flags.intersects(
                        TitleFlags::Locked | TitleFlags::ShadowHidden | TitleFlags::Removed,
                    )
                    && t.votes - t.downvotes > -1
            })
            .map(|t| t.into_with_db(&db.db))
            .collect::<Vec<_>>(),
    ))
}

#[get("/titles/broken", wrap = "ETagCache")]
async fn get_broken_titles(db_lock: DBLock) -> JsonResult<Vec<ApiTitle>> {
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(
        db.db
            .titles
            .iter()
            .rev()
            .filter(|t| t.flags.contains(TitleFlags::MissingVotes))
            .map(|t| t.into_with_db(&db.db))
            .collect::<Vec<_>>(),
    ))
}

#[get("/titles/uuid/{uuid}", wrap = "ETagCache")]
async fn get_title_by_uuid(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<ApiTitle> {
    let Some(uuid) = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.into_inner().as_str())
        .cloned()
    else {
        return Err(EmptyStatus(StatusCode::NOT_FOUND));
    };
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(
        db.db
            .titles
            .iter()
            .find(|t| Arc::ptr_eq(&t.uuid, &uuid))
            .map(|t| t.into_with_db(&db.db))
            .ok_or(EmptyStatus(StatusCode::NOT_FOUND))?,
    ))
}

#[get("/titles/video_id/{video_id}", wrap = "ETagCache")]
async fn get_titles_by_video_id(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<Vec<ApiTitle>> {
    let video_id = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.into_inner().as_str())
        .cloned();
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    let titles = match video_id {
        None => vec![],
        Some(id) => db
            .db
            .titles
            .iter()
            .rev()
            .filter(|title| Arc::ptr_eq(&title.video_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    Ok(web::Json(titles))
}

#[get("/titles/user_id/{user_id}", wrap = "ETagCache")]
async fn get_titles_by_user_id(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<Vec<ApiTitle>> {
    let user_id = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.into_inner().as_str())
        .cloned();
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    let titles = match user_id {
        None => vec![],
        Some(id) => db
            .db
            .titles
            .iter()
            .rev()
            .filter(|title| Arc::ptr_eq(&title.user_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    Ok(web::Json(titles))
}

#[get("/titles/channel/{channel}", wrap = "ETagCache")]
async fn get_titles_by_channel(
    db_lock: DBLock,
    path: web::Path<String>,
) -> JsonResultOrFetchProgress<Vec<ApiTitle>> {
    let channel_cache = {
        let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
        db.channel_cache.clone()
    };
    let channel_data = channel_cache
        .get_channel(path.into_inner().as_str())
        .await
        .context("Failed to get channel info")?;

    match channel_data {
        GetChannelOutput::Pending(progress) => {
            let mut resp = web::Json(api::ChannelFetchProgress::from(&progress)).extend();
            resp.extensions.insert(ETagCacheControl::DoNotCache);
            Ok(Either::Right((resp, *NOT_READY_YET)))
        }
        GetChannelOutput::Resolved(result) => {
            // we only really need the string pointer's address to figure out if they're equal, thanks to
            // the `StringSet`
            let vid_set: HashSet<usize> = result.video_ids.iter().map(utils::arc_addr).collect();
            let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
            let titles = db
                .db
                .titles
                .iter()
                .rev()
                .filter(|title| vid_set.contains(&utils::arc_addr(&title.video_id)))
                .map(|t| t.into_with_db(&db.db))
                .collect();
            Ok(Either::Left(web::Json(titles)))
        }
    }
}

#[get("/thumbnails", wrap = "ETagCache")]
async fn get_thumbnails(
    db_lock: DBLock,
    query: web::Query<MainEndpointURLParams>,
) -> JsonResult<Vec<ApiThumbnail>> {
    if query.count > 1024 {
        return Err(
            anyhow!(
                ("Too many requested thumbnails. You requested {} titles, but the configured max is 1024.", query.count),
                extend: extensions::status::BAD_REQUEST.clone()
            ).into()
        );
    }
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(
        db.db
            .thumbnails
            .iter()
            .rev()
            .skip(query.offset)
            .take(query.count)
            .map(|t| t.into_with_db(&db.db))
            .collect::<Vec<_>>(),
    ))
}

#[get("/thumbnails/broken", wrap = "ETagCache")]
async fn get_broken_thumbnails(db_lock: DBLock) -> JsonResult<Vec<ApiThumbnail>> {
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(
        db.db
            .thumbnails
            .iter()
            .rev()
            .filter(|t| {
                t.flags
                    .intersects(ThumbnailFlags::MissingVotes | ThumbnailFlags::MissingTimestamp)
            })
            .map(|t| t.into_with_db(&db.db))
            .collect::<Vec<_>>(),
    ))
}

#[get("/thumbnails/uuid/{uuid}", wrap = "ETagCache")]
async fn get_thumbnail_by_uuid(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<ApiThumbnail> {
    let Some(uuid) = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.into_inner().as_str())
        .cloned()
    else {
        return Err(EmptyStatus(StatusCode::NOT_FOUND));
    };
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(
        db.db
            .thumbnails
            .iter()
            .find(|t| Arc::ptr_eq(&t.uuid, &uuid))
            .map(|t| t.into_with_db(&db.db))
            .ok_or(EmptyStatus(StatusCode::NOT_FOUND))?,
    ))
}

#[get("/thumbnails/video_id/{video_id}", wrap = "ETagCache")]
async fn get_thumbnails_by_video_id(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<Vec<ApiThumbnail>> {
    let video_id = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.into_inner().as_str())
        .cloned();
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    let titles = match video_id {
        None => vec![],
        Some(id) => db
            .db
            .thumbnails
            .iter()
            .rev()
            .filter(|thumb| Arc::ptr_eq(&thumb.video_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    Ok(web::Json(titles))
}

#[get("/thumbnails/user_id/{video_id}", wrap = "ETagCache")]
async fn get_thumbnails_by_user_id(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<Vec<ApiThumbnail>> {
    let user_id = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.into_inner().as_str())
        .cloned();
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    let titles = match user_id {
        None => vec![],
        Some(id) => db
            .db
            .thumbnails
            .iter()
            .rev()
            .filter(|thumb| Arc::ptr_eq(&thumb.user_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    Ok(web::Json(titles))
}

#[get("/thumbnails/channel/{channel}", wrap = "ETagCache")]
async fn get_thumbnails_by_channel(
    db_lock: DBLock,
    path: web::Path<String>,
) -> JsonResultOrFetchProgress<Vec<ApiThumbnail>> {
    let channel_cache = {
        let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
        db.channel_cache.clone()
    };
    let channel_data = channel_cache
        .get_channel(path.into_inner().as_str())
        .await
        .context("Failed to get channel info")?;

    match channel_data {
        GetChannelOutput::Pending(progress) => {
            let mut resp = web::Json(api::ChannelFetchProgress::from(&progress)).extend();
            resp.extensions.insert(ETagCacheControl::DoNotCache);
            Ok(Either::Right((resp, *NOT_READY_YET)))
        }
        GetChannelOutput::Resolved(result) => {
            // we only really need the string pointer's address to figure out if they're equal, thanks to
            // the `StringSet`
            let vid_set: HashSet<usize> = result.video_ids.iter().map(utils::arc_addr).collect();
            let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
            let thumbs = db
                .db
                .thumbnails
                .iter()
                .rev()
                .filter(|thumbnail| vid_set.contains(&utils::arc_addr(&thumbnail.video_id)))
                .map(|t| t.into_with_db(&db.db))
                .collect();
            Ok(Either::Left(web::Json(thumbs)))
        }
    }
}

#[get("/users/user_id/{user_id}", wrap = "ETagCache")]
async fn get_user_by_userid(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<User> {
    let user_id = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.as_str())
        .cloned();
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(match user_id {
        None => User {
            user_id: path.into_inner().into(),
            username: None,
            username_locked: false,
            vip: false,
            title_count: 0,
            thumbnail_count: 0,
            warning_count: 0,
            active_warning_count: 0,
            last_submission: None,
            title_submission_rate: None,
            thumbnail_submission_rate: None,
        },
        Some(user_id) => {
            let username = db.db.get_username(&user_id);
            User::from_db(&db.db, &user_id, username)
        }
    }))
}

#[get("/users/username/{username}", wrap = "ETagCache")]
async fn get_users_by_username(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<Vec<User>> {
    let username = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.into_inner().as_str())
        .cloned();
    let Some(username) = username else {
        return Ok(web::Json(vec![]));
    };

    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(
        db.db
            .usernames
            .iter()
            .filter(|u| Arc::ptr_eq(&u.username, &username))
            .map(|u| User::from_db(&db.db, &u.user_id, Some(u)))
            .collect(),
    ))
}

#[get("/warnings", wrap = "ETagCache")]
async fn get_warnings(db_lock: DBLock,

    query: web::Query<MainEndpointURLParams>,
) -> JsonResult<Vec<ApiWarning>> {
    if query.count > 1024 {
        return Err(
            anyhow!(
                ("Too many requested titles. You requested {} titles, but the configured max is 1024.", query.count),
                extend: extensions::status::BAD_REQUEST.clone()
            ).into()
        );
    }
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;

    Ok(web::Json(
        db.db
            .warnings
            .iter()
            .rev()
            .skip(query.offset)
            .take(query.count)
            .map(|w| w.into_with_db(&db.db))
            .collect(),
    ))
}

#[get("/warnings/user_id/{user_id}/received", wrap = "ETagCache")]
async fn get_user_warnings(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<Vec<ApiWarning>> {
    let user_id = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.as_str())
        .cloned();
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(match user_id {
        None => vec![],
        Some(user_id) => db
            .db
            .warnings
            .iter()
            .rev()
            .filter(|w| Arc::ptr_eq(&w.warned_user_id, &user_id))
            .map(|w| w.into_with_db(&db.db))
            .collect(),
    }))
}

#[get("/warnings/user_id/{user_id}/issued", wrap = "ETagCache")]
async fn get_issued_warnings(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<Vec<ApiWarning>> {
    let user_id = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.as_str())
        .cloned();
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(match user_id {
        None => vec![],
        Some(user_id) => db
            .db
            .warnings
            .iter()
            .rev()
            .filter(|w| Arc::ptr_eq(&w.issuer_user_id, &user_id))
            .map(|w| w.into_with_db(&db.db))
            .collect(),
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
async fn get_video(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<Video> {
    let video_id = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.as_str())
        .cloned();
    Ok(web::Json(match video_id {
        None => unknown_video(path.as_str().into()),
        Some(video_id) => {
            let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
            let video_info = db.db.get_video_info(&video_id);
            match video_info {
                None => unknown_video(video_id),
                Some(video_info) => Video {
                    random_thumbnail: get_random_time_for_video(&video_id, Some(video_info)),
                    video_id,
                    duration: Some(video_info.video_duration),
                    fraction_unmarked: video_info.uncut_segments.iter().map(|s| s.length).sum(),
                    has_outro: video_info.has_outro,
                },
            }
        }
    }))
}

#[get("/casual_titles", wrap = "ETagCache")]
async fn get_casual_titles(
    db_lock: DBLock,
    query: web::Query<MainEndpointURLParams>,
) -> JsonResult<Vec<ApiCasualTitle>> {
    if query.count > 1024 {
        return Err(
            anyhow!(
                ("Too many requested casual titles. You requested {} titles, but the configured max is 1024.", query.count),
                extend: extensions::status::BAD_REQUEST.clone()
            ).into()
        );
    }
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    Ok(web::Json(
        db.db
            .casual_titles
            .iter()
            .rev()
            .skip(query.offset)
            .take(query.count)
            .map(From::from)
            .collect::<Vec<_>>(),
    ))
}

#[get("/casual_titles/video_id/{video_id}", wrap = "ETagCache")]
async fn get_casual_titles_by_video_id(
    db_lock: DBLock,
    string_set: StringSetLock,
    path: web::Path<String>,
) -> JsonResult<Vec<ApiCasualTitle>> {
    let video_id = string_set
        .read()
        .map_err(|_| SS_READ_ERR.clone())?
        .set
        .get(path.into_inner().as_str())
        .cloned();
    let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
    let titles = match video_id {
        None => vec![],
        Some(id) => db
            .db
            .casual_titles
            .iter()
            .rev()
            .filter(|t| Arc::ptr_eq(&t.video_id, &id))
            .map(From::from)
            .collect(),
    };
    Ok(web::Json(titles))
}

#[get("/casual_titles/channel/{channel}", wrap = "ETagCache")]
async fn get_casual_titles_by_channel(
    db_lock: DBLock,
    path: web::Path<String>,
) -> JsonResultOrFetchProgress<Vec<ApiCasualTitle>> {
    let channel_cache = {
        let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
        db.channel_cache.clone()
    };
    let channel_data = channel_cache
        .get_channel(path.into_inner().as_str())
        .await
        .context("Failed to get channel info")?;

    match channel_data {
        GetChannelOutput::Pending(progress) => {
            let mut resp = web::Json(api::ChannelFetchProgress::from(&progress)).extend();
            resp.extensions.insert(ETagCacheControl::DoNotCache);
            Ok(Either::Right((resp, *NOT_READY_YET)))
        }
        GetChannelOutput::Resolved(result) => {
            // we only really need the string pointer's address to figure out if they're equal, thanks to
            // the `StringSet`
            let vid_set: HashSet<usize> = result.video_ids.iter().map(utils::arc_addr).collect();
            let db = db_lock.read().map_err(|_| DB_READ_ERR.clone())?;
            let thumbs = db
                .db
                .casual_titles
                .iter()
                .rev()
                .filter(|t| vid_set.contains(&utils::arc_addr(&t.video_id)))
                .map(From::from)
                .collect();
            Ok(Either::Left(web::Json(thumbs)))
        }
    }
}
