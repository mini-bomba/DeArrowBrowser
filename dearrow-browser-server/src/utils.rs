use std::fmt::{Debug, Display};

use actix_web::{ResponseError, http::{StatusCode, header::ContentType}, HttpResponse};
pub enum Error {
    Anyhow(anyhow::Error),
    AnyhowStatus {status: StatusCode, error: anyhow::Error},
    EmptyStatus(StatusCode),
}

impl Error {
    pub fn set_status(self, status: StatusCode) -> Error {
        match self {
            Error::Anyhow(error) => Error::AnyhowStatus { status, error },
            Error::AnyhowStatus { error, .. } => Error::AnyhowStatus { status, error },
            Error::EmptyStatus(..) => Error::EmptyStatus(status),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Anyhow(ref err) => Debug::fmt(err, f),
            Error::AnyhowStatus {ref error, ..} => Debug::fmt(error, f),
            Error::EmptyStatus(status) => f.debug_tuple("Error::EmptyStatus").field(status).finish(),
        }
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Anyhow(ref err) => Display::fmt(err, f),
            Error::AnyhowStatus {ref error, ..} => Display::fmt(error, f),
            Error::EmptyStatus(status) => write!(f, "{status}"),
        }
    }
}
impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Error::Anyhow(value)
    }
}
impl std::error::Error for Error {}
impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Anyhow(..) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::AnyhowStatus { status, .. } => *status,
            Error::EmptyStatus(status) => *status,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let mut builder = HttpResponse::build(self.status_code());
        match self {
            Error::Anyhow(err) => builder.insert_header(ContentType::plaintext()).body(format!("{err:?}")),
            Error::AnyhowStatus { error, .. } => builder.insert_header(ContentType::plaintext()).body(format!("{error:?}")),
            Error::EmptyStatus(..) => builder.finish(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait MapInto<T> {
    fn map_into(self) -> T;
}
impl<T, U> MapInto<Option<T>> for Option<U>
where U: Into<T>
{
    fn map_into(self) -> Option<T> {
        self.map(Into::into)
    }
}

