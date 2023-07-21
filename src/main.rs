use std::{path::Path, sync::{RwLock, Mutex}, ops::DerefMut};
use actix_files::Files;
use actix_web::{HttpServer, App, web};
use anyhow::{Context, anyhow};
use chrono::Utc;
use dearrow_browser::{DearrowDB, StringSet};

mod utils;
mod routes;
mod state;
mod api_models;
use state::*;


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
                .configure(routes::configure_routes)
                .app_data(config.clone())
                .app_data(db.clone())
                .app_data(string_set.clone())
            )
            .service(Files::new("/", "static").index_file("index.html"))
    })
    .bind(("127.0.0.1", 9292)).context("Failed to bind to 127.0.0.1:9292")?
    .run()
    .await
    .context("Error while running the server")
}
