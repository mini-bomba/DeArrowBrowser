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
use actix_web::{http::header::EntityTag, rt::{spawn, time::sleep}, web};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use dearrow_parser::{DearrowDB, StringSet};
use error_handling::{bail, ErrContext, ErrorContext, ResContext};
use futures::{channel::oneshot, future::{BoxFuture, Shared}, lock::Mutex, select_biased, FutureExt};
use getrandom::getrandom;
use reqwest::Client;
use std::{collections::HashMap, path::PathBuf, sync::{atomic::AtomicUsize, Arc, RwLock}};
use serde::{Serialize, Deserialize};

use crate::{constants::*, innertube};

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
    pub enable_timings_header: bool,
    pub channel_cache_path: PathBuf,
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
            enable_timings_header: false,
            channel_cache_path: PathBuf::from("./cache/channels")
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
    pub video_info_count: usize,
    pub uncut_segment_count: usize,
}

impl DatabaseState {
    pub fn get_etag(&self) -> EntityTag {
        match &self.etag {
            None => self.generate_etag(),
            Some(ref t) => t.clone(),
        }
    }

    pub fn calculate_video_info_count(&self) -> usize {
        self.db.video_infos.iter().map(|chunk| chunk.len()).sum()
    }

    pub fn calculate_uncut_segment_count(&self) -> usize {
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
            self.video_info_count,
            self.uncut_segment_count,
        ))
    }
}

type UCIDFutureResult = Result<Arc<str>, ErrorContext>;
type SharedUCIDFuture = Shared<BoxFuture<'static, UCIDFutureResult>>;

#[derive(Clone)]
pub struct ChannelCache {
    /// NOTE: Keys of this hashmap and results from these futures are NOT stored in the `StringSet`!
    handle_to_ucid_cache: Arc<Mutex<HashMap<Arc<str>, SharedUCIDFuture>>>,
    /// NOTE: Keys of this hashmap are NOT stored in the `StringSet`!
    data_cache: Arc<Mutex<HashMap<Arc<str>, ChannelDataCacheEntry>>>,
    string_set: Arc<RwLock<StringSet>>,
    client: reqwest::Client,
}

#[derive(Clone)]
enum ChannelDataCacheEntry {
    Pending {
        future: Shared<oneshot::Receiver<Result<Arc<ChannelData>, ErrorContext>>>,
        progress: Arc<ChannelFetchProgress>,
    },
    Resolved(Arc<ChannelData>),
    Failed(ErrorContext),
}

#[derive(Debug, Default)]
pub struct ChannelFetchProgress {
    pub videos_fetched: AtomicUsize,
    pub videos_in_fscache: AtomicUsize,
}

#[derive(Debug)]
pub struct ChannelData {
    pub channel_name: Box<str>,
    /// only contains video ids found in the `StringSet` at the time of creation
    pub video_ids: Box<[Arc<str>]>,
    pub total_videos: usize,
}

#[derive(Clone, Debug)]
pub enum GetChannelOutput {
    Pending(Arc<ChannelFetchProgress>),
    Resolved(Arc<ChannelData>),
}

impl ChannelCache {
    pub fn new(string_set: Arc<RwLock<StringSet>>, client: Client) -> ChannelCache {
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

    pub async fn num_channels_cached(&self) -> usize {
        self.data_cache.lock().await.len()
    }

    async fn handle_to_ucid(client: Client, handle: Arc<str>) -> UCIDFutureResult {
        innertube::handle_to_ucid(&client, &handle).await.map(Into::into)
    }

    async fn fetch_channel(&self, ucid: &str, progress: Arc<ChannelFetchProgress>) -> Result<Arc<ChannelData>, ErrorContext> {
        let it_res = innertube::get_channel(&self.client, ucid, progress).await?;
        let string_set = self.string_set.read().map_err(|_| SS_READ_ERR.clone())?;
        Ok(Arc::new(ChannelData {
            channel_name: it_res.name.into(),
            total_videos: it_res.video_ids.len(),
            video_ids: it_res.video_ids.into_iter().filter_map(|vid| string_set.set.get(vid.as_str()).cloned()).collect(),
        }))
    }

    async fn fetch_channel_task(cache: ChannelCache, ucid: Arc<str>, progress: Arc<ChannelFetchProgress>, output: oneshot::Sender<Result<Arc<ChannelData>, ErrorContext>>) {
        let result = cache.fetch_channel(&ucid, progress).await;
        let _ = output.send(result.clone());
        let mut data_cache = cache.data_cache.lock().await;
        match result {
            Ok(res) => data_cache.insert(ucid, ChannelDataCacheEntry::Resolved(res)),
            Err(err) => data_cache.insert(ucid, ChannelDataCacheEntry::Failed(err))
        };
    }

    pub async fn get_channel(&self, handle: &str) -> Result<GetChannelOutput, ErrorContext> {
        let ucid = if UCID_REGEX.is_match(handle) {
            handle.into()
        } else {
            let handle = handle.to_lowercase();
            let handle = if HANDLE_REGEX.is_match(&handle) {
                handle.into()
            } else {
                let maybe_handle = format!("@{handle}");
                if !HANDLE_REGEX.is_match(&maybe_handle) {
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

        let channel_data_entry = {
            let mut channel_data_cache = self.data_cache.lock().await;
            channel_data_cache.entry(ucid).or_insert_with_key(|ucid| {
                let progress: Arc<ChannelFetchProgress> = Arc::default();
                let (sender, receiver) = oneshot::channel();
                spawn(Self::fetch_channel_task(self.clone(), ucid.clone(), progress.clone(), sender));
                ChannelDataCacheEntry::Pending { 
                    future: receiver.shared(),
                    progress,
                }
            }).clone()
        };

        Ok::<usize, ErrorContext>(0)?;

        match channel_data_entry {
            ChannelDataCacheEntry::Resolved(res) => Ok(GetChannelOutput::Resolved(res)),
            ChannelDataCacheEntry::Failed(err) => Err(err.context("Failed to fetch channel data")),
            ChannelDataCacheEntry::Pending { mut future, progress } => select_biased! {
                res = future => Ok(GetChannelOutput::Resolved(res.context("Failed to fetch channel data")?.context("Failed to fetch channel data")?)),
                () = sleep(IT_TIMEOUT).fuse() => Ok(GetChannelOutput::Pending(progress)),
            }
        }
    }
}
