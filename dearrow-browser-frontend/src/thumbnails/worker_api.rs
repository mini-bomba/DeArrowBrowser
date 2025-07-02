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

use std::{error::Error, fmt::Display, rc::Rc};

use bincode::{Decode, Encode};

use super::common::{ThumbgenStats, ThumbnailKey};

pub const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq)]
pub enum RemoteThumbnailGenerationError {
    JSError{
        name: Option<Rc<str>>,
        message: Rc<str>,
        cause: Option<Rc<str>>,
        stack: Option<Rc<str>>,
    },
    ServerError(Rc<str>),
    UnexpectedStatusCode(u16),
    SilentFailure,
    ZeroSizeBlob,
    UnexpectedType(Rc<str>),
}

impl Display for RemoteThumbnailGenerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSError { name, message, .. } => write!(f, "A JS error has occurred: {}{message}", if let Some(n) = name { format!("{n}: ") } else {String::new()}),
            Self::ServerError(reason) => write!(f, "The server refused to generate the thumbnail: {reason}"),
            Self::UnexpectedStatusCode(code) => write!(f, "The server had sent an unexpected status code: {code}"),
            Self::SilentFailure => write!(f, "The server refused to generate the thumbnail, but no reason was given"),
            Self::ZeroSizeBlob => write!(f, "The server had sent an empty thumbnail"),
            Self::UnexpectedType(r#type) => write!(f, "The server sent a file of an unexpected type: {type} (expected 'image/*')"),
        }
    }
}

impl Error for RemoteThumbnailGenerationError {}

#[derive(Encode, Decode, Debug)]
pub enum ThumbnailWorkerRequest {
    Version {
        version: String,
        git_hash: Option<String>,
        git_dirty: Option<bool>,
    },
    BlobLinkDropped {
        ref_id: u16,
    },
    GetThumbnail {
        key: ThumbnailKey,
    },
    SettingUpdated {
        setting: WorkerSetting,
    },
    ClearErrors,
    GetStats,
    Ping,
    Disconnecting,
}

#[derive(Encode, Decode, Debug)]
pub enum ThumbnailWorkerResponse {
    Version {
        version: String,
        git_hash: Option<String>,
        git_dirty: Option<bool>,
    },
    DeserializationError {
        received_data: Vec<u8>,
    },
    Thumbnail {
        r#ref: Result<RawRemoteRef, RemoteThumbnailGenerationError>,
    },
    Stats {
        stats: ThumbgenStats,
    },
    Ok,
}

#[derive(Encode, Decode, Debug)]
pub struct RawRemoteRef {
    pub url: Box<str>,
    pub ref_id: u16,
}

#[derive(Encode, Decode, Debug)]
pub enum WorkerSetting {
    ThumbgenBaseUrl(String),
}

#[derive(Encode, Decode, Debug)]
pub struct ThumbnailWorkerRequestMessage {
    pub id: u16,
    pub request: ThumbnailWorkerRequest,
}

#[derive(Encode, Decode, Debug)]
pub struct ThumbnailWorkerResponseMessage {
    pub id: u16,
    pub response: ThumbnailWorkerResponse,
}
