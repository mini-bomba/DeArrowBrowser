/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2025 mini_bomba
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
use std::{ffi::CString, fmt::{Debug, Display}, fs, mem::MaybeUninit, ops::{Deref, DerefMut}, os::{fd::AsRawFd, unix::ffi::OsStrExt}, path::{Path, PathBuf}, sync::Arc, time::{SystemTime, UNIX_EPOCH}};

use actix_web::{dev::Extensions, http::{header::{HeaderMap, TryIntoHeaderPair}, StatusCode}, HttpResponse, Responder, ResponseError};
use base64::prelude::{BASE64_URL_SAFE_NO_PAD, Engine};
use cloneable_errors::{ErrContext, ErrorContext, IntoErrorIterator, ResContext};
use serde::de::DeserializeOwned;
use tokio::fs::File;

/// This extension will be present on a response if the response contains
/// a [`cloneable_errors::SerializableError`] encoded as json
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
    let fd_cstr = CString::new(format!("/proc/self/fd/{}", file.as_raw_fd())).expect("Failed to create a path to file descriptor as CString");
    let res = unsafe {
        libc::linkat(libc::AT_FDCWD, fd_cstr.as_ptr(), libc::AT_FDCWD, path_cstr.as_ptr(), libc::AT_SYMLINK_FOLLOW)
    };
    if res == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

pub fn random_b64<const N: usize>() -> String {
    let mut buffer = [MaybeUninit::<u8>::uninit(); N];
    BASE64_URL_SAFE_NO_PAD.encode(getrandom::getrandom_uninit(&mut buffer).expect("Should be able to get random data"))
}

#[derive(Clone)]
pub enum TempFileType {
    UnnamedFile,
    FileInTmpDir{
        current_path: PathBuf,
    },
}

pub struct TemporaryFile {
    file: File,
    r#type: TempFileType,
    is_committed: bool,
    target_path: PathBuf,
}

impl TemporaryFile {
    pub async fn new(target_path: PathBuf, fallback_tmpdir: &Path) -> std::io::Result<TemporaryFile> {
        match Self::new_unnamed(&target_path).await {
            // filesystem doesnt support unnamed files, try another type
            Err(err) if err.raw_os_error() == Some(libc::EOPNOTSUPP) => (),
            // other error, report
            Err(err) => return Err(err),
            // it worked, awesome
            Ok(file) => return Ok(TemporaryFile {
                file,
                r#type: TempFileType::UnnamedFile,
                is_committed: false,
                target_path,
            }),
        };

        // unnamed file didn't work, fall back
        let tmp_file_path = fallback_tmpdir.join(random_b64::<64>());
        let file = File::options().write(true).create_new(true).open(&tmp_file_path).await?;
        Ok(TemporaryFile { 
            file, 
            r#type: TempFileType::FileInTmpDir { current_path: tmp_file_path },
            is_committed: false, 
            target_path, 
        })
    }

    async fn new_unnamed(target_path: &Path) -> std::io::Result<File> {
        match target_path.parent() {
            Some(dir) => File::options().write(true).custom_flags(libc::O_TMPFILE).open(dir).await,
            // fake a "not supported" error to be caught by ::new()
            None => Err(std::io::Error::from_raw_os_error(libc::EOPNOTSUPP)),  
        }
    }   

    pub async fn commit(&mut self) -> std::result::Result<(), ErrorContext> {
        if self.is_committed {
            return Ok(());
        }

        match &self.r#type {
            TempFileType::UnnamedFile => {
                match tokio::fs::remove_file(&self.target_path).await {
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => (), // fine
                    Err(err) => return Err(err.context(format!("Failed to remove existing file at {}", self.target_path.display()))),
                    Ok(()) => (),
                };
                link_file(&self.file, &self.target_path).with_context(|| format!("Failed to link in the new file at {}", self.target_path.display()))?;
            },
            TempFileType::FileInTmpDir { current_path } => {
                tokio::fs::rename(&current_path, &self.target_path).await.with_context(|| format!("Failed to move temp file from {} to {}", current_path.display(), self.target_path.display()))?;
            },
        }
        self.is_committed = true;
        Ok(())
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if !self.is_committed {
            if let TempFileType::FileInTmpDir { current_path } = &self.r#type {
                let _ = std::fs::remove_file(current_path);  // can't do anything at this point
            }
        }
    }
}

impl Deref for TemporaryFile {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl DerefMut for TemporaryFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

#[allow(clippy::cast_possible_wrap)] // can't do anything really
#[allow(dead_code)]
pub fn unix_timestamp() -> i128 {
    match SystemTime::UNIX_EPOCH.elapsed() {
        Ok(d) => d.as_millis() as i128,
        Err(e) => -(e.duration().as_millis() as i128)
    }
}

pub trait ReqwestResponseExt {
    async fn json_debug<T: DeserializeOwned>(self, name: &'static str) -> std::result::Result<T, ErrorContext>;
}

impl ReqwestResponseExt for reqwest::Response {
    async fn json_debug<T: DeserializeOwned>(self, name: &'static str) -> std::result::Result<T, ErrorContext> {
        let status = self.status();
        let body = self.bytes().await.context("Failed to receive response")?;
        let decoded = serde_json::from_slice(&body);

        if (decoded.is_err() && std::env::var_os("DAB_IT_DUMP_ERRORS").is_some()) || std::env::var_os("DAB_IT_DUMP_ALL").is_some() {
            let mut path = PathBuf::from("./debug");
            path.push(format!("{}-{name}.json", unix_timestamp()));
            let dump_result: std::io::Result<()> = async {
                tokio::fs::create_dir_all(&path).await?;
                tokio::fs::write(&path, &body).await?;
                Ok(())
            }.await;
            if let Err(e) = dump_result {
                log::warn!("Failed to dump contents of a response: {e}");
            } else {
                return decoded.with_context(|| format!("Failed to decode '{status}' response as JSON - body dumped to {}", path.to_string_lossy()));
            }
        }

        decoded.with_context(|| format!("Failed to decode '{status}' response as JSON"))
    }
} 
