use serde::Serialize;

#[derive(Serialize)]
pub struct StatusResponse {
    pub last_updated: i64,
    pub updating_now: bool,
    pub titles: usize,
    pub thumbnails: usize,
    pub errors: usize,
    pub last_error: Option<String>,
}

pub type ErrorList = Vec<String>;
