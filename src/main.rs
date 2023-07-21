use std::{path::Path, sync::{RwLock, Mutex}, ops::DerefMut};
use actix_files::Files;
use actix_web::{HttpServer, App, Responder, get, web};
use anyhow::{Error, Context, anyhow};
use serde::Serialize;
use chrono::Utc;
use dearrow_browser::{DearrowDB, StringSet};

mod utils;

struct AppConfig {
    mirror_path: Box<Path>,
}

struct DatabaseState {
    db: DearrowDB,
    last_error: Option<Error>,
    errors: Box<[Error]>,
    last_updated: i64,
    updating_now: bool,
}

#[get("/")]
async fn helo() -> impl Responder {
    "hi"
}

#[derive(Serialize)]
struct StatusResponse {
    last_updated: i64,
    updating_now: bool,
    titles: usize,
    thumbnails: usize,
    errors: usize,
    last_error: Option<String>,
}

#[get("/status")]
async fn status(db_lock: web::Data<RwLock<DatabaseState>>) -> utils::Result<web::Json<StatusResponse>> {
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
async fn get_errors(db_lock: web::Data<RwLock<DatabaseState>>) -> utils::Result<web::Json<Vec<String>>> {
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(web::Json(db.errors.iter().map(|e| format!("{e:?}")).collect()))
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let config = web::Data::new(AppConfig {
        mirror_path: Path::new("/tmp").into(),
    });
    let string_set = web::Data::new(Mutex::new(StringSet::with_capacity(16384)));
    let (db, errors) = DearrowDB::load_dir(&config.mirror_path, string_set.lock().map_err(|_| anyhow!("Failed to aquire StringSet mutex due to poison"))?.deref_mut()).context("Initial DearrowDB load failed")?;

    let db: web::Data<RwLock<DatabaseState>> = web::Data::new(RwLock::new(DatabaseState {
        db,
        last_error: None,
        errors: errors.into(),
        last_updated: Utc::now().timestamp_millis(),
        updating_now: false
    }));

    HttpServer::new(move || {
        App::new()
            .service(web::scope("/api")
                .app_data(config.clone())
                .app_data(db.clone())
                .app_data(string_set.clone())
                .service(helo)
                .service(status)
                .service(get_errors)
            )
            .service(Files::new("/", "static").index_file("index.html"))
    })
    .bind(("127.0.0.1", 9292)).context("Failed to bind to 127.0.0.1:9292")?
    .run()
    .await
    .context("Error while running the server")
}
