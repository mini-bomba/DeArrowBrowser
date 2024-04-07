use std::sync::{RwLock, Arc};
use actix_web::{Responder, get, post, web, http::StatusCode, CustomizeResponder, HttpResponse, rt::task::spawn_blocking};
use anyhow::{anyhow, bail};
use chrono::{Utc, DateTime};
use dearrow_parser::{StringSet, DearrowDB, TitleFlags};
use dearrow_browser_api::*;
use log::warn;
use serde::Deserialize;

use crate::{utils::{self, IfNoneMatch}, state::*, built_info, etag_shortcircuit, etagged_json};

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(helo)
       .service(get_titles)
       .service(get_unverified_titles)
       .service(get_title_by_uuid)
       .service(get_titles_by_video_id)
       .service(get_titles_by_user_id)
       .service(get_thumbnails)
       .service(get_thumbnail_by_uuid)
       .service(get_thumbnails_by_video_id)
       .service(get_thumbnails_by_user_id)
       .service(get_user_by_userid)
       .service(get_status)
       .service(get_errors)
       .service(request_reload);
}

type JsonResult<T> = utils::Result<web::Json<T>>;
type CustomizedJsonResult<T> = utils::Result<CustomizeResponder<web::Json<T>>>;
type DBLock = web::Data<RwLock<DatabaseState>>;
type StringSetLock = web::Data<RwLock<StringSet>>;

#[derive(Deserialize)]
pub struct MainEndpointURLParams {
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_count")]
    pub count: usize,
}

#[inline(always)]
pub fn default_count() -> usize {
    50
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
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
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
        server_version: built_info::PKG_VERSION.into(),
        server_git_hash: built_info::GIT_COMMIT_HASH.map(|s| s.into()),
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
        let mut db_state = db_lock.write().map_err(|_| anyhow!("Failed to acquire DatabaseState for writing"))?;
        if db_state.updating_now {
            bail!("Already updating!");
        }
        db_state.updating_now = true;
    }
    warn!("Reload requested");
    let mut string_set_clone = string_set_lock.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?.clone();
    let (mut new_db, errors) = DearrowDB::load_dir(config.mirror_path.as_path(), &mut string_set_clone)?;
    new_db.sort();
    let last_updated = Utc::now().timestamp_millis();
    let last_modified = utils::get_mtime(&config.mirror_path.join("titles.csv"));
    {
        let mut string_set = string_set_lock.write().map_err(|_| anyhow!("Failed to acquire StringSet for writing"))?;
        let mut db_state = db_lock.write().map_err(|_| anyhow!("Failed to acquire DatabaseState for writing"))?;
        *string_set = string_set_clone;
        *db_state = DatabaseState {
            db: new_db,
            errors: errors.into(),
            last_updated,
            last_modified,
            updating_now: false,
            etag: None,
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
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(web::Json(db.errors.iter().map(|e| format!("{e:?}")).collect()))
}

#[get("/titles")]
async fn get_titles(db_lock: DBLock, inm: IfNoneMatch, query: web::Query<MainEndpointURLParams>) -> CustomizedJsonResult<Vec<ApiTitle>> {
    etag_shortcircuit!(db_lock, inm);
    if query.count > 1024 {
        return Err(
            utils::Error::from(anyhow!("Too many requested titles. You requested {} titles, but the configured max is 1024.", query.count))
                .set_status(StatusCode::BAD_REQUEST)
        );
    }
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(etagged_json!(db,
        db.db.titles.iter().rev().skip(query.offset).take(query.count)
            .map(|t| t.into_with_db(&db.db)).collect::<Vec<_>>()
    ))
}

#[get("/titles/unverified")]
async fn get_unverified_titles(db_lock: DBLock, inm: IfNoneMatch) -> CustomizedJsonResult<Vec<ApiTitle>> {
    etag_shortcircuit!(db_lock, inm);
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(etagged_json!(db, 
        db.db.titles.iter().rev()
            .filter(|t| t.flags.contains(TitleFlags::Unverified) && !t.flags.intersects(TitleFlags::Locked | TitleFlags::ShadowHidden | TitleFlags::Removed) && t.votes-t.downvotes > -1)
            .map(|t| t.into_with_db(&db.db)).collect::<Vec<_>>()
    ))
}

#[get("/titles/uuid/{uuid}")]
async fn get_title_by_uuid(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>, inm: IfNoneMatch) -> CustomizedJsonResult<ApiTitle> {
    etag_shortcircuit!(db_lock, inm);
    let Some(uuid) = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
            .set.get(path.into_inner().as_str()).cloned() else {
        return Err(utils::Error::EmptyStatus(StatusCode::NOT_FOUND));
    };
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(etagged_json!(db,
        db.db.titles.iter().find(|t| Arc::ptr_eq(&t.uuid, &uuid))
            .map(|t| t.into_with_db(&db.db))
            .ok_or(utils::Error::EmptyStatus(StatusCode::NOT_FOUND))?
    ))
}

#[get("/titles/video_id/{video_id}")]
async fn get_titles_by_video_id(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>, inm: IfNoneMatch) -> CustomizedJsonResult<Vec<ApiTitle>> {
    etag_shortcircuit!(db_lock, inm);
    let video_id = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
        .set.get(path.into_inner().as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    let titles = match video_id {
        None => vec![],
        Some(id) => db.db.titles.iter().rev()
            .filter(|title| Arc::ptr_eq(&title.video_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    let status = if titles.is_empty() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::OK
    };
    Ok(etagged_json!(db, titles).with_status(status))
}

#[get("/titles/user_id/{user_id}")]
async fn get_titles_by_user_id(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>, inm: IfNoneMatch) -> CustomizedJsonResult<Vec<ApiTitle>> {
    etag_shortcircuit!(db_lock, inm);
    let user_id = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
        .set.get(path.into_inner().as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    let titles = match user_id {
        None => vec![],
        Some(id) => db.db.titles.iter().rev()
            .filter(|title| Arc::ptr_eq(&title.user_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    let status = if titles.is_empty() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::OK
    };
    Ok(etagged_json!(db, titles).with_status(status))
}

#[get("/thumbnails")]
async fn get_thumbnails(db_lock: DBLock, inm: IfNoneMatch, query: web::Query<MainEndpointURLParams>) -> CustomizedJsonResult<Vec<ApiThumbnail>> {
    etag_shortcircuit!(db_lock, inm);
    if query.count > 1024 {
        return Err(
            utils::Error::from(anyhow!("Too many requested thumbnails. You requested {} thumnails, but the configured max is 1024.", query.count))
                .set_status(StatusCode::BAD_REQUEST)
        );
    }
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(etagged_json!(db,
        db.db.thumbnails.iter().rev().skip(query.offset).take(query.count)
            .map(|t| t.into_with_db(&db.db)).collect::<Vec<_>>()
    ))
}

#[get("/thumbnails/uuid/{uuid}")]
async fn get_thumbnail_by_uuid(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>, inm: IfNoneMatch) -> CustomizedJsonResult<ApiThumbnail> {
    etag_shortcircuit!(db_lock, inm);
    let Some(uuid) = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
            .set.get(path.into_inner().as_str()).cloned() else {
        return Err(utils::Error::EmptyStatus(StatusCode::NOT_FOUND));
    };
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(etagged_json!(db,
        db.db.thumbnails.iter().find(|t| Arc::ptr_eq(&t.uuid, &uuid))
            .map(|t| t.into_with_db(&db.db))
            .ok_or(utils::Error::EmptyStatus(StatusCode::NOT_FOUND))?
    ))
}

#[get("/thumbnails/video_id/{video_id}")]
async fn get_thumbnails_by_video_id(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>, inm: IfNoneMatch) -> CustomizedJsonResult<Vec<ApiThumbnail>> {
    etag_shortcircuit!(db_lock, inm);
    let video_id = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
        .set.get(path.into_inner().as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    let titles = match video_id {
        None => vec![],
        Some(id) => db.db.thumbnails.iter().rev()
            .filter(|thumb| Arc::ptr_eq(&thumb.video_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    let status = if titles.is_empty() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::OK
    };
    Ok(etagged_json!(db, titles).with_status(status))
}

#[get("/thumbnails/user_id/{video_id}")]
async fn get_thumbnails_by_user_id(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>, inm: IfNoneMatch) -> CustomizedJsonResult<Vec<ApiThumbnail>> {
    etag_shortcircuit!(db_lock, inm);
    let user_id = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
        .set.get(path.into_inner().as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    let titles = match user_id {
        None => vec![],
        Some(id) => db.db.thumbnails.iter().rev()
            .filter(|thumb| Arc::ptr_eq(&thumb.user_id, &id))
            .map(|t| t.into_with_db(&db.db))
            .collect(),
    };
    let status = if titles.is_empty() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::OK
    };
    Ok(etagged_json!(db, titles).with_status(status))
}

#[get("/users/user_id/{user_id}")]
async fn get_user_by_userid(db_lock: DBLock, string_set: StringSetLock, path: web::Path<String>, inm: IfNoneMatch) -> CustomizedJsonResult<User> {
    etag_shortcircuit!(db_lock, inm);
    let user_id = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
        .set.get(path.as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(etagged_json!(db, match user_id {
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

