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
use actix_web::http::header::EntityTag;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use dearrow_parser::DearrowDB;
use anyhow::Error;
use getrandom::getrandom;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub mirror_path: PathBuf,
    pub static_content_path: PathBuf,
    pub listen: ListenConfig,
    pub auth_secret: String,
    #[serde(default)]
    pub enable_sbserver_emulation: bool,
    #[serde(skip, default="Utc::now")]
    pub startup_timestamp: DateTime<Utc>
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut buffer: Vec<u8> = (0..32).map(|_| 0u8).collect();
        getrandom(&mut buffer[..]).unwrap();
        Self {
            mirror_path: PathBuf::from("./mirror"),
            static_content_path: PathBuf::from("./static"),
            listen: ListenConfig::default(),
            auth_secret: URL_SAFE_NO_PAD.encode(buffer),
            enable_sbserver_emulation: false,
            startup_timestamp: Utc::now(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListenConfig {
    pub tcp: Option<(String, u16)>,
    pub unix: Option<String>,
    pub unix_mode: Option<u32>,
}

impl Default for ListenConfig {
    fn default() -> Self {
        Self {
            tcp: Some(("0.0.0.0".to_owned(), 9292)),
            unix: None,
            unix_mode: None,
        }
    }
}

pub struct DatabaseState {
    pub db: DearrowDB,
    pub errors: Box<[Error]>,
    pub last_updated: i64,
    pub last_modified: i64,
    pub updating_now: bool,
    pub etag: Option<EntityTag>
}

impl DatabaseState {
    pub fn get_etag(&self) -> EntityTag {
        match &self.etag {
            None => self.generate_etag(),
            Some(ref t) => t.clone(),
        }
    }

    pub fn video_info_count(&self) -> usize {
        self.db.video_infos.iter().map(|chunk| chunk.len()).sum()
    }

    pub fn uncut_segment_count(&self) -> usize {
        self.db.video_infos.iter().map(|chunk| chunk.iter().map(|v| v.uncut_segments.len()).sum::<usize>()).sum()
    }

    pub fn generate_etag(&self) -> EntityTag {
        EntityTag::new_weak(format!(
            "{}:{}:{}+{}+{}+{}+{}+{}",
            self.last_updated, 
            self.last_modified, 
            self.db.titles.len(), 
            self.db.thumbnails.len(), 
            self.db.usernames.len(), 
            self.db.vip_users.len(),
            self.video_info_count(),
            self.uncut_segment_count(),
        ))
    }
}
