use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use dearrow_browser::DearrowDB;
use anyhow::Error;
use getrandom::getrandom;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub mirror_path: PathBuf,
    pub listen: ListenConfig,
    pub auth_secret: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut buffer: Vec<u8> = (0..32).map(|_| 0u8).collect();
        getrandom(&mut buffer[..]).unwrap();
        Self {
            mirror_path: PathBuf::from("./mirror"),
            listen: ListenConfig::default(),
            auth_secret: URL_SAFE_NO_PAD.encode(buffer),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListenConfig {
    pub ip: String,
    pub port: u16,
}

impl Default for ListenConfig {
    fn default() -> Self {
        Self {
            ip: "0.0.0.0".to_owned(),
            port: 9292,
        }
    }
}

pub struct DatabaseState {
    pub db: DearrowDB,
    pub last_error: Option<Error>,
    pub errors: Box<[Error]>,
    pub last_updated: i64,
    pub updating_now: bool,
}

