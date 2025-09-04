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

use std::rc::Rc;

use gloo_console::warn;

use crate::{
    worker_api::{StatsType, WorkerRequest, WorkerResponse},
    worker_client::{Error, WorkerClient},
    yt_metadata::common::{MetadataCacheStats, VideoMetadata},
};

#[derive(Clone, PartialEq, Eq)]
pub struct RemoteMetadataCache {
    pub client: WorkerClient,
}

impl RemoteMetadataCache {
    /// Retrieves the given video from the metadata cache or fetches it if it's not present
    pub async fn get_metadata(&self, video_id: Rc<str>) -> Result<VideoMetadata, Error> {
        match self
            .client
            .request(WorkerRequest::GetMetadata { video_id })
            .await?
        {
            WorkerResponse::Metadata { data } => data.map_err(Error::Serializable),
            _ => Err(Error::ProtocolError),
        }
    }

    /// Clears all cached errors
    ///
    /// Returns the number of cleared entries
    pub fn clear_errors(&self) {
        if let Err(e) = self.client.post_request(WorkerRequest::ClearMetadataErrors) {
            warn!(format!(
                "Failed to request worker to clear metadata cached errors: {e:?}"
            ));
        }
    }

    /// Clears all cached entries, except pending requests
    ///
    /// Returns the number of cleared entries
    pub fn clear_cache(&self) {
        if let Err(e) = self.client.post_request(WorkerRequest::ClearMetadataCache) {
            warn!(format!(
                "Failed to request worker to clear metadata cached entries: {e:?}"
            ));
        }
    }

    pub async fn get_stats(&self) -> Result<MetadataCacheStats, Error> {
        match self
            .client
            .request(WorkerRequest::GetStats {
                r#type: StatsType::Metadata,
            })
            .await?
        {
            WorkerResponse::MetadataStats { stats } => Ok(stats),
            _ => Err(Error::ProtocolError),
        }
    }
}
