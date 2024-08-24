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
use actix_web::{http::header::EntityTag, web};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use dearrow_parser::{DearrowDB, StringSet};
use error_handling::{bail, ErrorContext, ResContext};
use futures::{future::{BoxFuture, Shared}, lock::Mutex, FutureExt};
use getrandom::getrandom;
use reqwest::Client;
use std::{collections::HashMap, path::PathBuf, sync::{Arc, RwLock}};
use serde::{Serialize, Deserialize};

use crate::{constants, innertube};

pub type DBLock = web::Data<RwLock<DatabaseState>>;
pub type StringSetLock = web::Data<RwLock<StringSet>>;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub mirror_path: PathBuf,
    pub static_content_path: PathBuf,
    pub listen: ListenConfig,
    pub auth_secret: String,
    pub enable_sbserver_emulation: bool,
    pub reqwest_timeout_secs: f64,
    #[serde(skip)]
    pub startup_timestamp: DateTime<Utc>,
    pub innertube: InnertubeConfig,
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
            reqwest_timeout_secs: 20.,
            startup_timestamp: Utc::now(),
            innertube: InnertubeConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct InnertubeConfig {
    pub enable: bool,
    pub visitor_data: Option<String>,
    pub po_token: Option<String>,
}

impl Default for InnertubeConfig {
    fn default() -> Self {
        Self {
            enable: true,
            visitor_data: None,
            po_token: None,
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
    pub errors: Box<[ErrorContext]>,
    pub last_updated: i64,
    pub last_modified: i64,
    pub updating_now: bool,
    pub etag: Option<EntityTag>,
    pub channel_cache: ChannelCache,
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

type UCIDFutureResult = Result<Arc<str>, ErrorContext>;
type SharedUCIDFuture = Shared<BoxFuture<'static, UCIDFutureResult>>;

type ChannelFutureResult = Result<Arc<ChannelData>, ErrorContext>;
type SharedChannelFuture = Shared<BoxFuture<'static, ChannelFutureResult>>;

#[derive(Clone)]
pub struct ChannelCache {
    /// NOTE: Keas of this hashmap and results from these futures are NOT stored in the `StringSet`!
    handle_to_ucid_cache: Arc<Mutex<HashMap<Arc<str>, SharedUCIDFuture>>>,
    /// NOTE: Keys of this hashmap are NOT stored in the `StringSet`!
    data_cache: Arc<Mutex<HashMap<Arc<str>, SharedChannelFuture>>>,
    string_set: Arc<RwLock<StringSet>>,
    client: Arc<reqwest::Client>,
}

#[derive(Debug)]
pub struct ChannelData {
    pub channel_name: Box<str>,
    /// only contains video ids found in the `StringSet` at the time of creation
    pub video_ids: Box<[Arc<str>]>,
    pub total_videos: usize,
}

impl ChannelCache {
    pub fn new(string_set: Arc<RwLock<StringSet>>, client: Arc<Client>) -> ChannelCache {
        ChannelCache { 
            handle_to_ucid_cache: Arc::default(),
            data_cache: Arc::default(), 
            string_set,
            client,
        }
    }

    pub fn reset(&self) -> ChannelCache {
        ChannelCache { 
            handle_to_ucid_cache: Arc::default(),
            data_cache: Arc::default(), 
            string_set: self.string_set.clone(),
            client: self.client.clone(),
        }
    }

    async fn handle_to_ucid(client: Arc<Client>, handle: Arc<str>) -> UCIDFutureResult {
        innertube::handle_to_ucid(&client, &handle).await.map(Into::into)
    }

    async fn ucid_to_channel(client: Arc<Client>, string_set_lock: Arc<RwLock<StringSet>>, ucid: Arc<str>) -> ChannelFutureResult {
        let it_res = innertube::get_channel(&client, &ucid).await?;
        let string_set = string_set_lock.read().map_err(|_| constants::SS_READ_ERR.clone())?;
        Ok(Arc::new(ChannelData {
            channel_name: it_res.name.into(),
            total_videos: it_res.video_ids.len(),
            video_ids: it_res.video_ids.into_iter().filter_map(|vid| string_set.set.get(vid.as_str()).cloned()).collect(),
        }))
    }

    pub async fn get_channel(&self, handle: &str) -> ChannelFutureResult {
        let ucid = if innertube::UCID_REGEX.is_match(handle) {
            handle.into()
        } else {
            let handle = handle.to_lowercase();
            let handle = if innertube::HANDLE_REGEX.is_match(&handle) {
                handle.into()
            } else {
                let maybe_handle = format!("@{handle}");
                if !innertube::HANDLE_REGEX.is_match(&maybe_handle) {
                    bail!("Invalid handle/UCID!")
                }
                maybe_handle.into()
            };

            let ucid_future = {
                let mut ucid_cache = self.handle_to_ucid_cache.lock().await;
                ucid_cache.entry(handle).or_insert_with_key(|handle| Self::handle_to_ucid(self.client.clone(), handle.clone()).boxed().shared()).clone()
            };

            ucid_future.await.context("Failed to convert handle to UCID")?
        };

        let channel_data = {
            let mut channel_data_cache = self.data_cache.lock().await;
            channel_data_cache.entry(ucid).or_insert_with_key(|ucid| Self::ucid_to_channel(self.client.clone(), self.string_set.clone(), ucid.clone()).boxed().shared()).clone()
        }.await.context("Failed to fetch channel data")?;

        Ok(channel_data)
    }
}
