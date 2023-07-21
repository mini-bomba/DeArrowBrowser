use std::{sync::RwLock, ops::DerefMut, fs::File, io::{Read, Write, self}};
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

const CONFIG_PATH: &str = "config.toml";


#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let config: web::Data<AppConfig> = web::Data::new(match File::open(CONFIG_PATH) {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents).with_context(|| format!("Failed to read {CONFIG_PATH}"))?;
            toml::from_str(&contents).with_context(|| format!("Failed to deserialize contents of {CONFIG_PATH}"))?
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let cfg = AppConfig::default();
            let serialized = toml::to_string(&cfg).context("Failed to serialize default AppConfig as TOML")?;
            let mut file = File::options().write(true).create_new(true).open(CONFIG_PATH).with_context(|| format!("Failed to create {CONFIG_PATH}"))?;
            write!(file, "{serialized}").with_context(|| format!("Failed to write serialized default AppConfig to {CONFIG_PATH}"))?;
            cfg
        },
        Err(e) => {
            return Err(e).context(format!("Failed to open {CONFIG_PATH}"));
        }
    });
    let string_set = web::Data::new(RwLock::new(StringSet::with_capacity(16384)));
    let (db, errors) = DearrowDB::load_dir(&config.mirror_path, string_set.write().map_err(|_| anyhow!("Failed to aquire StringSet lock for writing"))?.deref_mut()).context("Initial DearrowDB load failed")?;

    let db: web::Data<RwLock<DatabaseState>> = web::Data::new(RwLock::new(DatabaseState {
        db,
        last_error: None,
        errors: errors.into(),
        last_updated: Utc::now().timestamp_millis(),
        updating_now: false
    }));

    let config_copy = config.clone();
    HttpServer::new(move || {
        App::new()
            .service(web::scope("/api")
                .configure(routes::configure_routes)
                .app_data(config_copy.clone())
                .app_data(db.clone())
                .app_data(string_set.clone())
            )
            .service(Files::new("/", "static").index_file("index.html"))
    })
    .bind((config.listen.ip.as_str(), config.listen.port)).with_context(|| format!("Failed to bind to {}:{}", config.listen.ip.as_str(), config.listen.port))?
    .run()
    .await
    .context("Error while running the server")
}
