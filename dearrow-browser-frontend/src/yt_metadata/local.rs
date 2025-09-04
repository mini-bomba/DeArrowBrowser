/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2025 mini_bomba
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

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use cloneable_errors::{ErrorContext, ResContext};
use futures::{future::{LocalBoxFuture, Shared}, FutureExt};
use serde::Deserialize;

use crate::{
    constants::YOUTUBE_OEMBED_URL, utils_common::RcEq, utils_common::api_request, yt_metadata::common::{youtu_be_link, MetadataCacheStats, VideoMetadata}
};

#[derive(Deserialize)]
struct OEmbedResponse {
    pub title: Rc<str>,
    pub author_url: String,
}

/// Youtube metadata cache owned by the current thread/worker/window
#[derive(Clone, PartialEq, Eq)]
pub struct LocalMetadataCache {
    inner: RcEq<InnerMetaCache>,
}

#[derive(Default)]
struct InnerMetaCache {
    metadata: RefCell<HashMap<Rc<str>, FetchState>>,
}

#[derive(Clone)]
enum FetchState {
    Pending(Shared<LocalBoxFuture<'static, Result<VideoMetadata, ErrorContext>>>),
    Ready(VideoMetadata),
    Failed(ErrorContext),
}

async fn fetch_metadata(vid: &str) -> Result<VideoMetadata, ErrorContext> {
    let mut url = YOUTUBE_OEMBED_URL.clone();
    url.query_pairs_mut()
        .clear()
        .append_pair("url", youtu_be_link(vid).as_str());
    let response: OEmbedResponse = api_request(url).await.context("oembed request failed")?;
    let channel = reqwest::Url::parse(&response.author_url)
        .context("Failed to parse channel URL")?
        .path_segments()
        .context("Failed to extract channel handle from URL: not a base???")?
        .filter(|s| !s.is_empty())
        .next_back()
        .map(Rc::from)
        .context("Failed to extract channel handle from URL: URL has no segments")?;

    Ok(VideoMetadata {
        title: response.title,
        channel,
    })
}

impl LocalMetadataCache {
    #[allow(clippy::new_without_default)] // would be too easy, these should be reused
    pub fn new() -> LocalMetadataCache {
        LocalMetadataCache {
            inner: RcEq(Rc::default()),
        }
    }

    /// Internal function that fetches metadata and updates the corresponding entry in cache
    /// Results are returned directly
    async fn fetch_metadata(self, video_id: Rc<str>) -> Result<VideoMetadata, ErrorContext> {
        let result = fetch_metadata(&video_id).await;
        self.inner.metadata.borrow_mut().insert(video_id, match result.clone() {
            Ok(v) => FetchState::Ready(v),
            Err(e) => FetchState::Failed(e),
        });
        result
    }

    /// Retrieves the given video from the metadata cache or fetches it if it's not present
    pub async fn get_metadata(&self, video_id: Rc<str>) -> Result<VideoMetadata, ErrorContext> {
        let entry = self
            .inner
            .metadata
            .borrow_mut()
            .entry(video_id)
            .or_insert_with_key(|key| {
                FetchState::Pending(
                    self.clone()
                        .fetch_metadata(key.clone())
                        .boxed_local()
                        .shared(),
                )
            })
            .clone();
        match entry {
            FetchState::Pending(f) => f.await,
            FetchState::Ready(v) => Ok(v),
            FetchState::Failed(e) => Err(e),
        }
    }

    /// Clears all cached errors
    ///
    /// Returns the number of cleared entries
    pub fn clear_errors(&self) -> usize {
        let mut cache = self.inner.metadata.borrow_mut();
        let before = cache.len();
        cache.retain(|_, v| !matches!(v, FetchState::Failed(..)));
        before - cache.len()
    }

    /// Clears all cached entries, except pending requests
    ///
    /// Returns the number of cleared entries
    pub fn clear_cache(&self) -> usize {
        let mut cache = self.inner.metadata.borrow_mut();
        let before = cache.len();
        cache.retain(|_, v| matches!(v, FetchState::Pending(..)));
        before - cache.len()
    }

    pub fn get_stats(&self) -> MetadataCacheStats {
        self.inner.metadata.borrow().values().fold(MetadataCacheStats::default(), |mut stats, v| {
            stats.total += 1;
            match v {
                FetchState::Pending(..) => {
                    stats.pending += 1;
                },
                FetchState::Ready(..) => {
                    stats.cached += 1;
                },
                FetchState::Failed(..) => {
                    stats.failed += 1;
                }
            }
            stats
        })
    }
}
