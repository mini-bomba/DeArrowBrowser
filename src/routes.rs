use std::sync::RwLock;
use actix_web::{Responder, get, web};
use anyhow::anyhow;
use super::{utils, state::*, api_models::*};

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(helo)
       .service(status)
       .service(get_errors);
}

type JsonResult<T> = utils::Result<web::Json<T>>;

#[get("/")]
async fn helo() -> impl Responder {
    "hi"
}

#[get("/status")]
async fn status(db_lock: web::Data<RwLock<DatabaseState>>) -> JsonResult<StatusResponse> {
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(web::Json(StatusResponse {
        last_updated: db.last_updated,
        updating_now: db.updating_now,
        titles: db.db.titles.len(),
        thumbnails: db.db.thumbnails.len(),
        errors: db.errors.len(),
        last_error: db.last_error.as_ref().map(|e| format!("{e:?}"))
    }))
}

#[get("/errors")]
async fn get_errors(db_lock: web::Data<RwLock<DatabaseState>>) -> JsonResult<ErrorList> {
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(web::Json(db.errors.iter().map(|e| format!("{e:?}")).collect()))
}
