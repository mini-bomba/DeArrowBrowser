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

use std::fmt::{Debug, Display};
use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use cloneable_errors::{ErrorContext, IntoErrorIterator};

use crate::{errors::extensions::{empty_body::EmptyBody, status::ResponseCodeExt}, middleware::timings::NoTimings};

pub mod extensions;

/// This extension will be present on a response if the response contains
/// a [`cloneable_errors::SerializableError`] encoded as json
pub struct SerializableErrorResponseMarker;

pub enum Error {
    #[allow(clippy::enum_variant_names)]
    ErrorContext(ErrorContext),
    EmptyStatus(StatusCode),
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ErrorContext(ref err) => Debug::fmt(err, f),
            Error::EmptyStatus(status) => f.debug_tuple("Error::EmptyStatus").field(status).finish(),
        }
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ErrorContext(ref err) => Display::fmt(err, f),
            Error::EmptyStatus(status) => write!(f, "{status}"),
        }
    }
}
impl From<ErrorContext> for Error {
    fn from(value: ErrorContext) -> Self {
        Error::ErrorContext(value)
    }
}
impl std::error::Error for Error {}
impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        // try using the response code extension
        match self {
            Error::ErrorContext(err) => {
                if let Some(ext) = err.find_extension::<ResponseCodeExt>() {
                    ext.0
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            },
            Error::EmptyStatus(status) => *status,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let mut builder = HttpResponse::build(self.status_code());
        match self {
            Error::ErrorContext(err) => {
                if err.find_extension::<NoTimings>().is_some() {
                    builder.extensions_mut().insert(NoTimings);
                }
                if err.find_extension::<EmptyBody>().is_some() {
                    return builder.finish();
                }
                builder.extensions_mut().insert(SerializableErrorResponseMarker);
                builder.json(err.serializable_copy())
            },
            Error::EmptyStatus(..) => builder.finish(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
