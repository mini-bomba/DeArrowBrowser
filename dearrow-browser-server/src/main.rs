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
use std::{sync::RwLock, fs::{File, Permissions, set_permissions}, io::{Read, Write, self}, os::unix::prelude::PermissionsExt};
use actix_files::{Files, NamedFile};
use actix_web::{HttpServer, App, web, dev::{ServiceResponse, fn_service, ServiceRequest}, middleware::NormalizePath};
use anyhow::{Context, anyhow, bail};
use chrono::Utc;
use env_logger::Env;
use log::info;
use dearrow_parser::{DearrowDB, StringSet};

mod utils;
mod routes;
mod state;
mod sbserver_emulation;
mod middleware;
use state::*;

const CONFIG_PATH: &str = "config.toml";


#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    let config: web::Data<AppConfig> = web::Data::new(match File::open(CONFIG_PATH) {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents).with_context(|| format!("Failed to read {CONFIG_PATH}"))?;
            let cfg: AppConfig = toml::from_str(&contents).with_context(|| format!("Failed to deserialize contents of {CONFIG_PATH}"))?;
            if cfg.listen.tcp.is_none() && cfg.listen.unix.is_none() {
                bail!("Invalid configuration - no tcp port or unix socket path specified");
            }
            cfg
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
    info!("Loading database...");
    let string_set = web::Data::new(RwLock::new(StringSet::with_capacity(16384)));
    let db: web::Data<RwLock<DatabaseState>> = {
        let mut string_set = string_set.write().map_err(|_| anyhow!("Failed to aquire StringSet lock for writing"))?;
        let (db, errors) = DearrowDB::load_dir(&config.mirror_path, &mut string_set).context("Initial DearrowDB load failed")?;
        string_set.clean();

        let mut db_state = DatabaseState {
            db,
            errors: errors.into(),
            last_updated: Utc::now().timestamp_millis(),
            last_modified: utils::get_mtime(&config.mirror_path.join("titles.csv")),
            updating_now: false,
            etag: None,
        };
        db_state.db.sort();
        db_state.etag = Some(db_state.generate_etag());
        web::Data::new(RwLock::new(db_state))
    };
    info!("Database ready!");

    let mut server = {
        let config = config.clone();
        HttpServer::new(move || {
            let config2 = config.clone();
            let mut app = App::new()
                .wrap(NormalizePath::trim())
                .app_data(config.clone())
                .app_data(db.clone())
                .app_data(string_set.clone())
                .service(web::scope("/api")
                    .configure(routes::configure)
                );
            if config.enable_sbserver_emulation {
                app = app.service(web::scope("/sbserver")
                    .configure(sbserver_emulation::configure_enabled)
                );
            } else {
                app = app.service(web::scope("/sbserver")
                    .configure(sbserver_emulation::configure_disabled)
                );
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
                    }))
            )
        })
    };
    if let Some((ref ip, port)) = config.listen.tcp {
        let ip_str = ip.as_str();
        server = server.bind((ip_str, port)).with_context(|| format!("Failed to bind to tcp port {ip_str}:{port}"))?;
        info!("Listening on {ip_str}:{port}");
    };
    if let Some(ref path) = config.listen.unix {
        let path_str = path.as_str();
        server = server.bind_uds(path_str).with_context(|| format!("Failed to bind to unix socket {path_str}"))?;
        if let Some(mode) = config.listen.unix_mode {
            let perms = Permissions::from_mode(mode);
            set_permissions(path_str, perms).with_context(|| format!("Failed to change mode of unix socket {path_str} to {mode}"))?;
        }
        info!("Listening on {path_str}");
    };
    server.run()
    .await
    .context("Error while running the server")
}

mod built_info {
    // Contents generated by buildscript, using built
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
