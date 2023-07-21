use std::fmt::{Debug, Display};

use actix_web::ResponseError;
pub struct Error(anyhow::Error);

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}
impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Error(value)
    }
}
impl std::error::Error for Error {}
impl ResponseError for Error {}
pub type Result<T> = std::result::Result<T, Error>;
