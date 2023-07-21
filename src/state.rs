use dearrow_browser::DearrowDB;
use anyhow::Error;
use std::path::Path;

pub struct AppConfig {
    pub mirror_path: Box<Path>,
}

pub struct DatabaseState {
    pub db: DearrowDB,
    pub last_error: Option<Error>,
    pub errors: Box<[Error]>,
    pub last_updated: i64,
    pub updating_now: bool,
}

