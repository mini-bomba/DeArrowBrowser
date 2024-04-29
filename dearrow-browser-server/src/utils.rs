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
use std::{fmt::{Debug, Display}, path::Path, fs, time::UNIX_EPOCH, future::{Ready, ready}};

use actix_web::{ResponseError, http::{StatusCode, header::{ContentType, self, Header, EntityTag}}, HttpResponse, FromRequest};
use sha2::{Sha256, Digest, digest::{typenum::U32, generic_array::GenericArray}};


pub enum Error {
    Anyhow(anyhow::Error, StatusCode),
    EmptyStatus(StatusCode),
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Anyhow(ref err, _) => Debug::fmt(err, f),
            Error::EmptyStatus(status) => f.debug_tuple("Error::EmptyStatus").field(status).finish(),
        }
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Anyhow(ref err, _) => Display::fmt(err, f),
            Error::EmptyStatus(status) => write!(f, "{status}"),
        }
    }
}
impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Error::Anyhow(value, StatusCode::INTERNAL_SERVER_ERROR)
    }
}
impl std::error::Error for Error {}
impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        let (Error::Anyhow(_, status) | Error::EmptyStatus(status)) = self;
        *status
    }

    fn error_response(&self) -> HttpResponse {
        let mut builder = HttpResponse::build(self.status_code());
        match self {
            Error::Anyhow(err, _) => builder.insert_header(ContentType::plaintext()).body(format!("{err:?}")),
            Error::EmptyStatus(..) => builder.finish(),
        }
    }
}

impl Error {
    pub fn set_status(self, status: StatusCode) -> Self {
        match self {
            Error::Anyhow(err, _) => Error::Anyhow(err, status),
            Error::EmptyStatus(..) => Error::EmptyStatus(status),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn sha256(s: &str) -> GenericArray<u8, U32> {
    let mut hasher = Sha256::new();
    hasher.update(s);
    hasher.finalize()
}

pub fn get_mtime(p: &Path) -> i64 {
    fs::metadata(p).ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| d.as_millis().try_into().ok())
        .unwrap_or(0)
}

#[derive(Debug)]
pub struct IfNoneMatch(Vec<EntityTag>);

impl FromRequest for IfNoneMatch {
    type Error = actix_web::error::ParseError;
    type Future = Ready<std::result::Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        ready(header::IfNoneMatch::parse(req).map(|h| match h {
            header::IfNoneMatch::Any => IfNoneMatch(vec![]),
            header::IfNoneMatch::Items(v) => IfNoneMatch(v),
        }))
    }    
}

impl IfNoneMatch {
    pub fn any_match(&self, other: &EntityTag) -> bool {
        self.0.iter().any(|etag| etag.weak_eq(other))
    }

    pub fn shortcircuit(&self, other: &EntityTag) -> Result<()> {
        if self.any_match(other) {
            Err(Error::EmptyStatus(StatusCode::NOT_MODIFIED))
        } else {
            Ok(())
        }
    }
}

#[macro_export]
macro_rules! etag_shortcircuit {
    ($db_lock: expr, $inm: expr) => {
        let db = $db_lock.read().map_err(|_| anyhow::anyhow!("Failed to acquire DatabaseState for reading"))?;
        $inm.shortcircuit(&db.get_etag())?
    };
}

#[macro_export]
macro_rules! etagged_json {
    ($db: expr, $struct: expr) => {{
        use actix_web::Responder;
        actix_web::web::Json($struct).customize()
        .append_header(actix_web::http::header::ETag($db.get_etag()))
        .append_header(actix_web::http::header::CacheControl(vec![actix_web::http::header::CacheDirective::NoCache]))
    }};
}
