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
use actix_files::{Files, NamedFile};
use actix_web::{
    dev::{fn_service, ServiceRequest, ServiceResponse},
    middleware::NormalizePath,
    web, App, HttpResponse, HttpServer,
};
use chrono::Utc;
use constants::CONFIG_PATH;
use dearrow_parser::{db::DearrowDB, dedupe::StringSet};
use env_logger::Env;
use cloneable_errors::{bail, ErrorContext, ResContext};
use log::info;
use std::{
    fs::{create_dir_all, set_permissions, File, Permissions},
    future::ready,
    io::{self, Read, Write},
    os::unix::prelude::PermissionsExt,
    sync::RwLock,
    time::Duration,
};

mod constants;
mod errors;
mod innertube;
mod middleware;
mod routes;
mod sbserver_emulation;
mod state;
mod utils;
use reqwest::ClientBuilder;
use state::*;

#[actix_web::main]
async fn main() -> Result<(), ErrorContext> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    let config: web::Data<AppConfig> = web::Data::new(match File::open(CONFIG_PATH) {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .with_context(|| format!("Failed to read {CONFIG_PATH}"))?;
            let cfg: AppConfig = toml::from_str(&contents)
                .with_context(|| format!("Failed to deserialize contents of {CONFIG_PATH}"))?;
            if cfg.listen.tcp.is_none() && cfg.listen.unix.is_none() {
                bail!("Invalid configuration - no tcp port or unix socket path specified");
            }
            cfg
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let cfg = AppConfig::default();
            let serialized =
                toml::to_string(&cfg).context("Failed to serialize default AppConfig as TOML")?;
            let mut file = File::options()
                .write(true)
                .create_new(true)
                .open(CONFIG_PATH)
                .with_context(|| format!("Failed to create {CONFIG_PATH}"))?;
            write!(file, "{serialized}").with_context(|| {
                format!("Failed to write serialized default AppConfig to {CONFIG_PATH}")
            })?;
            cfg
        }
        Err(e) => {
            return Err(e).context(format!("Failed to open {CONFIG_PATH}"));
        }
    });
    {
        create_dir_all(&config.cache_path).context("Failed to create the cache main directory")?;
        create_dir_all(config.cache_path.join(constants::FSCACHE_TEMPDIR))
            .context("Failed to create the cache temporary directory")?;
        create_dir_all(config.cache_path.join(constants::FSCACHE_PLAYLISTS))
            .context("Failed to create the cache playlists directory")?;
        create_dir_all(
            config
                .cache_path
                .join(constants::IT_BROWSE_VIDEOS.cache_dir),
        )
        .context("Failed to create the channel cache videos directory")?;
        create_dir_all(config.cache_path.join(constants::IT_BROWSE_LIVE.cache_dir))
            .context("Failed to create the channel cache vods directory")?;
        create_dir_all(
            config
                .cache_path
                .join(constants::IT_BROWSE_SHORTS.cache_dir),
        )
        .context("Failed to create the channel cache shorts directory")?;
    }
    info!("Loading database...");
    let string_set_lock = web::Data::new(RwLock::new(StringSet::with_capacity(16384)));
    let reqwest_client = web::ThinData(
        ClientBuilder::new()
            .timeout(Duration::from_secs_f64(config.reqwest_timeout_secs))
            .build()
            .expect("Should be able to create a reqwest Client"),
    );
    let db: web::Data<RwLock<DatabaseState>> = {
        let mut string_set = string_set_lock
            .write()
            .map_err(|_| constants::SS_WRITE_ERR.clone())?;
        let (db, errors) = DearrowDB::load_dir(&config.mirror_path, &mut string_set, false)
            .context("Initial DearrowDB load failed")?;
        string_set.clean();

        let mut db_state = DatabaseState {
            db,
            errors: errors.into(),
            last_updated: Utc::now().timestamp_millis(),
            last_modified: utils::get_mtime(&config.mirror_path.join("titles.csv")),
            updating_now: false,
            etag: None,
            channel_cache: ChannelCache::new(
                string_set_lock.clone().into_inner(),
                config.clone().into_inner(),
                reqwest_client.0.clone(),
            ),
            uncut_segment_count: 0,
            video_info_count: 0,
        };
        db_state.uncut_segment_count = db_state.calculate_uncut_segment_count();
        db_state.video_info_count = db_state.calculate_video_info_count();
        db_state.etag = Some(db_state.generate_etag());
        web::Data::new(RwLock::new(db_state))
    };
    info!("Skipped loading {} usernames", db.read().unwrap().db.usernames_skipped);
    info!("Database ready!");

    let mut server = {
        let config = config.clone();
        HttpServer::new(move || {
            let config2 = config.clone();
            let mut app = App::new()
                .wrap(NormalizePath::trim())
                .app_data(config.clone())
                .app_data(db.clone())
                .app_data(string_set_lock.clone())
                .app_data(reqwest_client.clone())
                .wrap(middleware::custom_status::CustomStatusCodes)
                .wrap(middleware::timings::Timings)
                .wrap(middleware::errors::ErrorRepresentation)
                .service(web::scope("/api").configure(routes::configure(config.clone())));
            if config.enable_sbserver_emulation {
                app = app.service(
                    web::scope("/sbserver").configure(sbserver_emulation::configure_enabled),
                );
            } else {
                app = app.service(
                    web::scope("/sbserver").configure(sbserver_emulation::configure_disabled),
                );
            }
            if config.innertube.enable {
                app = app.service(web::scope("/innertube").configure(innertube::configure_enabled));
            } else {
                app =
                    app.service(web::scope("/innertube").configure(innertube::configure_disabled));
            }
            if config.enable_fakeapi {
                app = app.service(web::scope("/fakeapi").default_service(fn_service(
                    |req: ServiceRequest| {
                        ready(Ok(ServiceResponse::new(
                            req.into_parts().0,
                            HttpResponse::Ok().finish(),
                        )))
                    },
                )));
            }
            app.service(
                Files::new("/", config.static_content_path.as_path())
                    .index_file("index.html")
                    .default_handler(fn_service(move |req: ServiceRequest| {
                        let config = config2.clone();
                        async move {
                            let (req, _) = req.into_parts();
                            let index_file = config.static_content_path.join("index.html");
                            let file = NamedFile::open_async(index_file.as_path()).await?;
                            let resp = file.into_response(&req);
                            Ok(ServiceResponse::new(req, resp))
                        }
                    })),
            )
        })
    };
    if let Some((ref ip, port)) = config.listen.tcp {
        let ip_str = ip.as_str();
        server = server
            .bind((ip_str, port))
            .with_context(|| format!("Failed to bind to tcp port {ip_str}:{port}"))?;
        info!("Listening on {ip_str}:{port}");
    }
    if let Some(ref path) = config.listen.unix {
        let path_str = path.as_str();
        server = server
            .bind_uds(path_str)
            .with_context(|| format!("Failed to bind to unix socket {path_str}"))?;
        if let Some(mode) = config.listen.unix_mode {
            let perms = Permissions::from_mode(mode);
            set_permissions(path_str, perms).with_context(|| {
                format!("Failed to change mode of unix socket {path_str} to {mode}")
            })?;
        }
        info!("Listening on {path_str}");
    }
    server.run().await.context("Error while running the server")
}

mod built_info {
    // Contents generated by buildscript, using built
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
