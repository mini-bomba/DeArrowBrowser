use std::sync::{RwLock, Arc};
use actix_web::{Responder, get, web, http::StatusCode, CustomizeResponder};
use anyhow::anyhow;
use dearrow_parser::StringSet;
use dearrow_browser_api::*;

use crate::{utils::{self, MapInto}, state::*};

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(helo)
       .service(get_title_by_uuid)
       .service(get_titles_by_video_id)
       .service(get_titles_by_user_id)
       .service(get_thumbnail_by_uuid)
       .service(get_thumbnails_by_video_id)
       .service(get_thumbnails_by_user_id)
       .service(get_status)
       .service(get_errors);
}

type JsonResult<T> = utils::Result<web::Json<T>>;
type CustomizedJsonResult<T> = utils::Result<CustomizeResponder<web::Json<T>>>;

#[get("/")]
async fn helo() -> impl Responder {
    "hi"
}

#[get("/status")]
async fn get_status(db_lock: web::Data<RwLock<DatabaseState>>, string_set: web::Data<RwLock<StringSet>>) -> JsonResult<StatusResponse> {
    let strings = match string_set.try_read() {
        Err(_) => None,
        Ok(set) => Some(set.set.len()),
    };
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(web::Json(StatusResponse {
        last_updated: db.last_updated,
        updating_now: db.updating_now,
        titles: db.db.titles.len(),
        thumbnails: db.db.thumbnails.len(),
        errors: db.errors.len(),
        last_error: db.last_error.as_ref().map(|e| format!("{e:?}")),
        string_count: strings,
    }))
}

#[get("/errors")]
async fn get_errors(db_lock: web::Data<RwLock<DatabaseState>>) -> JsonResult<ErrorList> {
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(web::Json(db.errors.iter().map(|e| format!("{e:?}")).collect()))
}

#[get("/titles/uuid/{uuid}")]
async fn get_title_by_uuid(db_lock: web::Data<RwLock<DatabaseState>>, path: web::Path<String>) -> JsonResult<ApiTitle> {
    let uuid = path.into_inner();
    Ok(web::Json(
        db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?
            .db.titles.get(uuid.as_str()).map_into()
            .ok_or(utils::Error::EmptyStatus(StatusCode::NOT_FOUND))?
    ))
}

#[get("/titles/video_id/{video_id}")]
async fn get_titles_by_video_id(db_lock: web::Data<RwLock<DatabaseState>>, string_set: web::Data<RwLock<StringSet>>, path: web::Path<String>) -> CustomizedJsonResult<Vec<ApiTitle>> {
    let video_id = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
        .set.get(path.into_inner().as_str()).cloned();
    let titles = match video_id {
        None => vec![],
        Some(id) => db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?
            .db.titles.values()
            .filter(|title| Arc::ptr_eq(&title.video_id, &id))
            .map(Into::into)
            .collect(),
    };
    let status = if titles.is_empty() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::OK
    };
    Ok(web::Json(titles).customize().with_status(status))
}

#[get("/titles/user_id/{user_id}")]
async fn get_titles_by_user_id(db_lock: web::Data<RwLock<DatabaseState>>, string_set: web::Data<RwLock<StringSet>>, path: web::Path<String>) -> CustomizedJsonResult<Vec<ApiTitle>> {
    let user_id = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
        .set.get(path.into_inner().as_str()).cloned();
    let titles = match user_id {
        None => vec![],
        Some(id) => db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?
            .db.titles.values()
            .filter(|title| Arc::ptr_eq(&title.user_id, &id))
            .map(Into::into)
            .collect(),
    };
    let status = if titles.is_empty() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::OK
    };
    Ok(web::Json(titles).customize().with_status(status))
}

#[get("/thumbnails/uuid/{uuid}")]
async fn get_thumbnail_by_uuid(db_lock: web::Data<RwLock<DatabaseState>>, path: web::Path<String>) -> JsonResult<ApiThumbnail> {
    let uuid = path.into_inner();
    Ok(web::Json(
        db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?
            .db.thumbnails.get(uuid.as_str()).map_into()
            .ok_or(utils::Error::EmptyStatus(StatusCode::NOT_FOUND))?
    ))
}

#[get("/thumbnails/video_id/{video_id}")]
async fn get_thumbnails_by_video_id(db_lock: web::Data<RwLock<DatabaseState>>, string_set: web::Data<RwLock<StringSet>>, path: web::Path<String>) -> CustomizedJsonResult<Vec<ApiThumbnail>> {
    let video_id = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
        .set.get(path.into_inner().as_str()).cloned();
    let titles = match video_id {
        None => vec![],
        Some(id) => db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?
            .db.thumbnails.values()
            .filter(|thumb| Arc::ptr_eq(&thumb.video_id, &id))
            .map(Into::into)
            .collect(),
    };
    let status = if titles.is_empty() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::OK
    };
    Ok(web::Json(titles).customize().with_status(status))
}

#[get("/thumbnails/user_id/{video_id}")]
async fn get_thumbnails_by_user_id(db_lock: web::Data<RwLock<DatabaseState>>, string_set: web::Data<RwLock<StringSet>>, path: web::Path<String>) -> CustomizedJsonResult<Vec<ApiThumbnail>> {
    let user_id = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
        .set.get(path.into_inner().as_str()).cloned();
    let titles = match user_id {
        None => vec![],
        Some(id) => db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?
            .db.thumbnails.values()
            .filter(|thumb| Arc::ptr_eq(&thumb.user_id, &id))
            .map(Into::into)
            .collect(),
    };
    let status = if titles.is_empty() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::OK
    };
    Ok(web::Json(titles).customize().with_status(status))
}
