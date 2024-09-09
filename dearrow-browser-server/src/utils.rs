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
use std::{ffi::CString, fmt::{Debug, Display}, fs, os::{fd::AsRawFd, unix::ffi::OsStrExt}, path::Path, sync::Arc, time::UNIX_EPOCH};

use actix_web::{dev::Extensions, http::{header::{HeaderMap, TryIntoHeaderPair}, StatusCode}, HttpResponse, Responder, ResponseError};
use error_handling::{ErrorContext, IntoErrorIterator};
use sha2::{Sha256, Digest, digest::{typenum::U32, generic_array::GenericArray}};

/// This extension will be present on a response if the response contains
/// a [`error_handling::SerializableError`] encoded as json
pub struct SerializableErrorResponseMarker;

pub enum Error {
    #[allow(clippy::enum_variant_names)]
    ErrorContext(ErrorContext, StatusCode),
    EmptyStatus(StatusCode),
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ErrorContext(ref err, _) => Debug::fmt(err, f),
            Error::EmptyStatus(status) => f.debug_tuple("Error::EmptyStatus").field(status).finish(),
        }
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ErrorContext(ref err, _) => Display::fmt(err, f),
            Error::EmptyStatus(status) => write!(f, "{status}"),
        }
    }
}
impl From<ErrorContext> for Error {
    fn from(value: ErrorContext) -> Self {
        Error::ErrorContext(value, StatusCode::INTERNAL_SERVER_ERROR)
    }
}
impl std::error::Error for Error {}
impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        let (Error::ErrorContext(_, status) | Error::EmptyStatus(status)) = self;
        *status
    }

    fn error_response(&self) -> HttpResponse {
        let mut builder = HttpResponse::build(self.status_code());
        match self {
            Error::ErrorContext(err, _) => {
                {
                    let mut exts = builder.extensions_mut();
                    exts.insert(SerializableErrorResponseMarker);
                }
                builder.json(err.serializable_copy())
            },
            Error::EmptyStatus(..) => builder.finish(),
        }
    }
}

impl Error {
    pub fn set_status(self, status: StatusCode) -> Self {
        match self {
            Error::ErrorContext(err, _) => Error::ErrorContext(err, status),
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

pub trait HeaderMapExt {
    fn append_header<H: TryIntoHeaderPair>(&mut self, header: H) -> std::result::Result<(), H::Error>;
    fn replace_header<H: TryIntoHeaderPair>(&mut self, header: H) -> std::result::Result<(), H::Error>;
}

impl HeaderMapExt for HeaderMap {
    fn append_header<H: TryIntoHeaderPair>(&mut self, header: H) -> std::result::Result<(), H::Error> {
        let (name, value) = header.try_into_pair()?;
        self.append(name, value);
        Ok(())
    }
    fn replace_header<H: TryIntoHeaderPair>(&mut self, header: H) -> std::result::Result<(), H::Error> {
        let (name, value) = header.try_into_pair()?;
        self.insert(name, value);
        Ok(())
    }
}

pub fn arc_addr<T: ?Sized>(arc: &Arc<T>) -> usize {
    Arc::as_ptr(arc).cast::<()>() as usize
}

pub struct ExtendResponder<T: Responder> {
    pub inner: T,
    pub extensions: Extensions,
}

impl<T: Responder> Responder for ExtendResponder<T> {
    type Body = T::Body;

    fn respond_to(self, req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        let mut resp = self.inner.respond_to(req);
        resp.extensions_mut().extend(self.extensions);
        resp
    }
}

pub trait ResponderExt: Responder + Sized {
    fn extend(self) -> ExtendResponder<Self> {
        ExtendResponder { inner: self, extensions: Extensions::new() }
    }
}

impl<T> ResponderExt for T where T: Responder + Sized {}


pub fn link_file<T: AsRawFd>(file: &T, new_path: &Path) -> std::io::Result<()> {
    let path_cstr = CString::new(new_path.as_os_str().as_bytes()).expect("Failed to convert new_path to a CString");
    let res = unsafe {
        libc::linkat(file.as_raw_fd(), c"".as_ptr(), libc::AT_FDCWD, path_cstr.as_ptr(), libc::AT_EMPTY_PATH)
    };
    if res == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
