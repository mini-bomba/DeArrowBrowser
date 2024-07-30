/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024 mini_bomba
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

use std::{cell::{Cell, OnceCell, RefCell}, collections::HashMap, rc::Rc};

use futures::{future::{LocalBoxFuture, Shared}, FutureExt};
use gloo_console::{debug, error};
use reqwest::Url;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{js_sys::{Error, Object, Reflect}, Blob};

use super::{common::{CacheStats, ThumbnailKey}, utils::*, worker_api::RemoteThumbnailGenerationError};

const SWEEP_INTERVAL: i32 = 30_000;

/// Represents ownership of the underlying object URL (aka bloblink)
///
/// Uncloneable, dropping will cause the underlying URL to be revoked to free memory
#[derive(Debug, PartialEq, Eq)]
pub struct LocalBlobLink {
    url: Box<str>,
}

impl Drop for LocalBlobLink {
    fn drop(&mut self) {
        if let Err(error) = web_sys::Url::revoke_object_url(&self.url) {
            error!(format!("Failed to revoke object URL '{}'", &self.url), error);
        }
    }
}

impl LocalBlobLink {
    /// Borrows the inner URL string.
    /// As the ``str`` representing the URL is cloneable, this function is marked unsafe.
    ///
    /// # Resource Lifetime
    /// ``LocalBlobLink`` must outlive any clones of the inner URL - the URL may be invalidated when this
    /// struct is dropped.
    pub fn inner_url(&self) -> &str {
        &self.url
    }
}

#[derive(Clone)]
pub enum ThumbnailGenerationError {
    JSError(JsValue),
    ServerError(Rc<str>),
    UnexpectedStatusCode(u16),
    SilentFailure,
    ZeroSizeBlob,
    UnexpectedType(Rc<str>),
}

impl From<ThumbnailGenerationError> for RemoteThumbnailGenerationError {
    fn from(value: ThumbnailGenerationError) -> Self {
        match value {
            ThumbnailGenerationError::JSError(js_error) => match js_error.dyn_into::<Error>() { 
                Ok(err) => RemoteThumbnailGenerationError::JSError { 
                    name: Some(String::from(err.name()).into()), 
                    message: String::from(err.message()).into(), 
                    cause: err.cause().as_string().map(Into::into), 
                    stack: Reflect::get(&err, &"stack".into()).ok().and_then(|s| s.as_string().map(Into::into)),
                },
                Err(value) => RemoteThumbnailGenerationError::JSError { 
                    name: None, 
                    message: match value.dyn_into::<Object>() {
                        Ok(obj) => String::from(obj.to_string()),
                        Err(value) => String::from(make_jsstring(&value)),
                    }.into(),
                    cause: None, 
                    stack: None, 
                },
            },
            ThumbnailGenerationError::ServerError(msg) => RemoteThumbnailGenerationError::ServerError(msg),
            ThumbnailGenerationError::UnexpectedStatusCode(code) => RemoteThumbnailGenerationError::UnexpectedStatusCode(code),
            ThumbnailGenerationError::SilentFailure => RemoteThumbnailGenerationError::SilentFailure,
            ThumbnailGenerationError::ZeroSizeBlob => RemoteThumbnailGenerationError::ZeroSizeBlob,
            ThumbnailGenerationError::UnexpectedType(r#type) => RemoteThumbnailGenerationError::UnexpectedType(r#type),
        }
    }
}

enum LocalThumbnailState {
    Pending(Shared<LocalBoxFuture<'static, Result<(), ThumbnailGenerationError>>>),
    Ready {
        thumbnail: Rc<LocalBlobLink>,
        eviction_timer: usize,
    },
    Failed(ThumbnailGenerationError),
}

/// Thumbnail "generator" & cache owned by the current thread/worker/window
#[derive(Clone)]
pub struct LocalThumbGenerator {
    inner: Rc<InnerLTG>,
}

impl PartialEq for LocalThumbGenerator {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
impl Eq for LocalThumbGenerator {}

struct InnerLTG {
    thumbs: RefCell<HashMap<ThumbnailKey, LocalThumbnailState>>,
    api_base_url: RefCell<Url>,
    eviction_threshold: Cell<usize>,
    sweep_interval: OnceCell<Interval<dyn FnMut()>>,
}

async fn generate_thumb(mut url: Url, key: &ThumbnailKey) -> Result<LocalBlobLink, ThumbnailGenerationError> {
    url.query_pairs_mut()
        .append_pair("videoID", &key.video_id)
        .append_pair("time", &key.timestamp);
    let response = GLOBAL.with(Clone::clone).fetch(url.as_str()).await.map_err(ThumbnailGenerationError::JSError)?;

    let status = response.status();
    if status != 200 {
        if status != 204 {
            return Err(ThumbnailGenerationError::UnexpectedStatusCode(status as u16));
        }

        let failure_reason = response.headers().get("X-Failure-Reason").map_err(ThumbnailGenerationError::JSError)?
            .ok_or(ThumbnailGenerationError::SilentFailure)?;
        return Err(ThumbnailGenerationError::ServerError(failure_reason.into()));
    }

    let blob: JsFuture = response.blob().map_err(ThumbnailGenerationError::JSError)?.into();
    let blob: Blob = blob.await.map_err(ThumbnailGenerationError::JSError)?.unchecked_into();

    if blob.size() <= 0. {
        return Err(ThumbnailGenerationError::ZeroSizeBlob);
    }

    let r#type = blob.type_();
    if !r#type.starts_with("image/") {
        return Err(ThumbnailGenerationError::UnexpectedType(r#type.into()));
    }

    let object_url = web_sys::Url::create_object_url_with_blob(&blob).map_err(ThumbnailGenerationError::JSError)?;
    let bloblink = LocalBlobLink { url: object_url.into() };
    Ok(bloblink)
}

impl Default for LocalThumbGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalThumbGenerator {
    pub fn new() -> LocalThumbGenerator {
        let inner = Rc::new(InnerLTG {
            thumbs: RefCell::new(HashMap::new()),
            api_base_url: RefCell::new(Url::parse("https://dearrow-thumb.minibomba.pro/api/v1/getThumbnail").unwrap()),
            eviction_threshold: Cell::new(5),
            sweep_interval: OnceCell::new(),
        });

        // start the sweeping task
        inner.sweep_interval.get_or_init(|| {
            let gen = Rc::downgrade(&inner);
            Interval::new(Closure::new(move || {
                let Some(gen) = gen.upgrade() else {
                    error!("LocalThumbGenerator sweep: Failed to upgrade InnerLTG ref!");
                    return;
                };
                let (incd, dropd) = LocalThumbGenerator { inner: gen }.sweep();
                if incd > 0 || dropd > 0 {
                    debug!(format!("Sweeped thumbgen cache - {incd} timers incremented, {dropd} entries dropped"));
                }
            }), SWEEP_INTERVAL)
        });

        LocalThumbGenerator { inner }
    }

    /// Internal coroutine that generates a thumbnail and updates its entry in the cache
    ///
    /// Errors are directly returned, but the thumbnail must be manually retrieved from the cache
    async fn generate_thumb(self, key: ThumbnailKey) -> Result<(), ThumbnailGenerationError> {
        let url = self.inner.api_base_url.borrow().clone();
        let new_state = match generate_thumb(url, &key).await {
            Ok(thumb) => LocalThumbnailState::Ready {
                thumbnail: thumb.into(),
                eviction_timer: 0,
            },
            Err(error) => LocalThumbnailState::Failed(error),
        };
        let result = if let LocalThumbnailState::Failed(ref error) = new_state {
            Err(error.clone())
        } else {
            Ok(())
        };
        self.inner.thumbs.borrow_mut().insert(key, new_state);
        result
    }

    /// Retrieves a given thumbnail from the cache or generates it if it isn't present in the
    /// cache.
    ///
    /// Errors are cached indefinetly, thumbnails might be freed after some time with no
    /// references.
    pub async fn get_thumb(&self, key: &ThumbnailKey) -> Result<Rc<LocalBlobLink>, ThumbnailGenerationError> {
        let future = match self.inner.thumbs.borrow_mut().entry(key.clone()).or_insert_with_key(|k| LocalThumbnailState::Pending(Self::generate_thumb(self.clone(), k.clone()).boxed_local().shared())) {
            LocalThumbnailState::Ready { thumbnail, ref mut eviction_timer, .. } => {
                *eviction_timer = 0;
                return Ok(thumbnail.clone())
            },
            LocalThumbnailState::Failed(err) => return Err(err.clone()),
            LocalThumbnailState::Pending(fut) => fut.clone(),
        };
        future.await?;
        sleep(50).await; // let firefox register the bloblink
        // At this point the thumbnail should be in the Ready state
        match self.inner.thumbs.borrow_mut().get_mut(key).expect("thumbnail should be Ready here") {
            LocalThumbnailState::Ready { thumbnail, ref mut eviction_timer, .. } => {
                *eviction_timer = 0;
                Ok(thumbnail.clone())
            },
            _ => panic!("thumbnail should be Ready here (but isn't?)"),
        }
    }

    /// Performs a sweep of the cache
    ///
    /// Any entries with only 1 strong reference on the ``Rc<LocalBlobLink>``
    /// will have their ``eviction_timer`` field incremented.
    ///
    /// When an ``eviction_timer`` reaches the threshold set by ``eviction_threshold``, the entry
    /// is dropped.
    ///
    /// Returns a tuple of (# of ``evicton_timer``s incremented, # of entries dropped)
    pub fn sweep(&self) -> (usize, usize) {
        let mut num_incremented = 0;
        let mut thumbs = self.inner.thumbs.borrow_mut();
        let before_sweep = thumbs.len();
        
        thumbs.retain(|_, v| {
            let LocalThumbnailState::Ready { ref thumbnail, ref mut eviction_timer } = v else { 
                return true;  // keep non-ready states
            };

            if Rc::strong_count(thumbnail) > 1 {
                // keep & reset timer
                *eviction_timer = 0;
                true
            } else {
                num_incremented += 1;
                *eviction_timer += 1;
                *eviction_timer < self.inner.eviction_threshold.get()
            }
        });

        (num_incremented, before_sweep-thumbs.len())
    }

    /// Clears all ``LocalThumbnailState::Failed`` entries
    ///
    /// Returns the number of cleared entries
    pub fn clear_errors(&self) -> usize {
        let mut thumbs = self.inner.thumbs.borrow_mut();
        let before = thumbs.len();
        thumbs.retain(|_, v| !matches!(v, LocalThumbnailState::Failed(..)));
        before-thumbs.len()
    }

    /// Replaces the current API url.
    /// Does not clear cache.
    ///
    /// The URL must point to the getThumbnail endpoint.
    pub fn set_api_url(&self, new_url: Url) {
        self.inner.api_base_url.replace(new_url);
    }

    /// Sets the new eviction threshold.
    /// Does not trigger a sweep.
    pub fn set_eviction_threshold(&self, new_threshold: usize) {
        self.inner.eviction_threshold.set(new_threshold);
    }

    /// Aggregates statistics about this thumbnail generator
    pub fn get_stats(&self) -> CacheStats {
        let thumbs = self.inner.thumbs.borrow();
        let mut res = CacheStats { 
            total: thumbs.len(),
            thumbs: 0,
            in_use: 0, 
            errors: 0, 
            pending: 0,
        };
        for state in thumbs.values() {
            match state {
                LocalThumbnailState::Pending(..) => res.pending += 1,
                LocalThumbnailState::Failed(..) => res.errors += 1,
                LocalThumbnailState::Ready { ref thumbnail, .. } => {
                    res.thumbs += 1;
                    if Rc::strong_count(thumbnail) > 1 {
                        res.in_use += 1;
                    }
                }
            }
        }
        res
    }
}
