use std::sync::Arc;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct StatusResponse {
    pub last_updated: i64,
    pub updating_now: bool,
    pub titles: usize,
    pub thumbnails: usize,
    pub errors: usize,
    pub last_error: Option<String>,
    pub string_count: Option<usize>,
}

pub type ErrorList = Vec<String>;

#[derive(Serialize, Deserialize, Clone)]
pub struct ApiTitle {
    pub uuid: Arc<str>,
    pub video_id: Arc<str>,
    pub title: Arc<str>,
    pub user_id: Arc<str>,
    pub time_submitted: i64,
    pub votes: i8,
    pub original: bool,
    pub locked: bool,
    pub shadow_hidden: bool,
    pub unverified: bool,
    pub score: i8,
}
#[cfg(feature = "dearrow-parser")]
impl From<&dearrow_parser::Title> for ApiTitle {
    fn from(value: &dearrow_parser::Title) -> Self {
        use dearrow_parser::TitleFlags;
        let unverified = value.flags.contains(TitleFlags::Unverified);
        Self { 
            uuid: value.uuid.clone(),
            video_id: value.video_id.clone(),
            title: value.title.clone(),
            user_id: value.user_id.clone(),
            time_submitted: value.time_submitted,
            votes: value.votes,
            original: value.flags.contains(TitleFlags::Original),
            locked: value.flags.contains(TitleFlags::Locked),
            shadow_hidden: value.flags.contains(TitleFlags::ShadowHidden),
            unverified,
            score: if unverified {
                value.votes - 1
            } else {
                value.votes
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ApiThumbnail {
    pub uuid: Arc<str>,
    pub video_id: Arc<str>,
    pub user_id: Arc<str>,
    pub time_submitted: i64,
    pub timestamp: Option<f64>,
    pub votes: i8,
    pub original: bool,
    pub locked: bool,
    pub shadow_hidden: bool,
}
#[cfg(feature = "dearrow-parser")]
impl From<&dearrow_parser::Thumbnail> for ApiThumbnail {
    fn from(value: &dearrow_parser::Thumbnail) -> Self {
        use dearrow_parser::ThumbnailFlags;
        Self {
            uuid: value.uuid.clone(),
            video_id: value.video_id.clone(),
            user_id: value.user_id.clone(),
            time_submitted: value.time_submitted,
            timestamp: value.timestamp,
            votes: value.votes,
            original: value.flags.contains(ThumbnailFlags::Original),
            locked: value.flags.contains(ThumbnailFlags::Locked),
            shadow_hidden: value.flags.contains(ThumbnailFlags::ShadowHidden),
        }
    }
}

