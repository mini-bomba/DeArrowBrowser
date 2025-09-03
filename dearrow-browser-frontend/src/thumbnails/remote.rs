/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024-2025 mini_bomba
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

use std::fmt::Debug;

use gloo_console::warn;

use super::common::{ThumbgenStats, ThumbnailKey};
use crate::{
    worker_api::{StatsType, WorkerRequest, WorkerResponse},
    worker_client::{Error, WorkerClient},
};

/// Represents a shared reference to the underlying object URL (aka bloblink)
///
/// The object URL is owned by a remote ``ThumbnailWorker``.
///
/// Uncloneable, dropping will notify the remote worker which may revoke the URL
pub struct RemoteBlobLink {
    client: WorkerClient,
    url: Box<str>,
    ref_id: u16,
}

impl Drop for RemoteBlobLink {
    fn drop(&mut self) {
        if let Err(err) = self.client.post_request(WorkerRequest::BlobLinkDropped { ref_id: self.ref_id }) {
            err.log("Failed to notify worker about a RemoteBlobLink being dropped");
        }
    }
}

impl Debug for RemoteBlobLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteBlobLink").field("url", &self.url).field("ref_id", &self.ref_id).finish_non_exhaustive()
    }
}

impl PartialEq for RemoteBlobLink {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}
impl Eq for RemoteBlobLink {}

impl RemoteBlobLink {
    /// Borrows the inner URL string.
    /// As the ``str`` representing the URL is cloneable, this function is marked unsafe.
    ///
    /// # Resource Lifetime
    /// ``RemoteBlobLink`` must outlive any clones of the inner URL - the URL may be invalidated when this
    /// struct is dropped.
    pub fn inner_url(&self) -> &str {
        &self.url
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RemoteThumbnailGenerator {
    pub client: WorkerClient,
}

impl RemoteThumbnailGenerator {
    pub async fn get_thumbnail(&self, key: ThumbnailKey) -> Result<RemoteBlobLink, Error> {
        let WorkerResponse::Thumbnail { r#ref } = self.client.request(WorkerRequest::GetThumbnail { key }).await? else {
            return Err(Error::ProtocolError);
        };
        let r#ref = r#ref.map_err(Error::Remote)?;
        Ok(RemoteBlobLink {
            client: self.client.clone(),
            ref_id: r#ref.ref_id,
            url: r#ref.url,
        })
    }

    /// Retrieves thumbgen statistics from the worker
    pub async fn get_stats(&self) -> Result<ThumbgenStats, Error> {
        let WorkerResponse::ThumbgenStats { stats } = self
            .client
            .request(WorkerRequest::GetStats {
                r#type: StatsType::Thumbgen,
            })
            .await?
        else {
            return Err(Error::ProtocolError);
        };
        Ok(stats)
    }

    pub fn clear_errors(&self) {
        if let Err(e) = self.client.post_request(WorkerRequest::ClearErrors) {
            warn!(format!("Failed to request worker to clear errors: {e}"));
        }
    }
}
