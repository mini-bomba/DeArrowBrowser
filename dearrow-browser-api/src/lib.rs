use std::sync::Arc;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct StatusResponse {
    pub last_updated: i64,
    pub last_modified: i64,
    pub updating_now: bool,
    pub titles: usize,
    pub thumbnails: usize,
    pub vip_users: usize,
    pub usernames: usize,
    pub errors: usize,
    pub string_count: Option<usize>,
    pub video_infos: usize,
    pub uncut_segments: usize,
    pub server_version: Arc<str>,
    pub server_git_hash: Option<Arc<str>>,
    pub server_git_dirty: Option<bool>,
    pub server_build_timestamp: Option<i64>,
    pub server_startup_timestamp: i64,
}

pub type ErrorList = Vec<String>;

#[cfg(feature = "dearrow-parser")]
pub trait IntoWithDatabase<T> {
    fn into_with_db(self, db: &dearrow_parser::DearrowDB) -> T;
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ApiTitle {
    pub uuid: Arc<str>,
    pub video_id: Arc<str>,
    pub title: Arc<str>,
    pub user_id: Arc<str>,
    pub time_submitted: i64,
    pub votes: i8,
    pub downvotes: i8,
    pub original: bool,
    pub locked: bool,
    pub shadow_hidden: bool,
    pub unverified: bool,
    pub removed: bool,
    pub score: i8,
    pub username: Option<Arc<str>>,
    pub vip: bool,
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
            downvotes: value.downvotes,
            original: value.flags.contains(TitleFlags::Original),
            locked: value.flags.contains(TitleFlags::Locked),
            shadow_hidden: value.flags.contains(TitleFlags::ShadowHidden),
            unverified,
            removed: value.flags.contains(TitleFlags::Removed),
            score: value.votes - value.downvotes - i8::from(unverified),
            username: None,
            vip: false,
        }
    }
}
#[cfg(feature = "dearrow-parser")]
impl IntoWithDatabase<ApiTitle> for &dearrow_parser::Title {
    fn into_with_db(self, db: &dearrow_parser::DearrowDB) -> ApiTitle {
        let mut res: ApiTitle = self.into();
        res.username = db.usernames.get(&res.user_id).map(|u| u.username.clone());
        res.vip = db.vip_users.contains(&res.user_id);
        res
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ApiThumbnail {
    pub uuid: Arc<str>,
    pub video_id: Arc<str>,
    pub user_id: Arc<str>,
    pub time_submitted: i64,
    pub timestamp: Option<f64>,
    pub votes: i8,
    pub downvotes: i8,
    pub original: bool,
    pub locked: bool,
    pub shadow_hidden: bool,
    pub removed: bool,
    pub score: i8,
    pub username: Option<Arc<str>>,
    pub vip: bool,
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
            downvotes: value.downvotes,
            original: value.flags.contains(ThumbnailFlags::Original),
            locked: value.flags.contains(ThumbnailFlags::Locked),
            shadow_hidden: value.flags.contains(ThumbnailFlags::ShadowHidden),
            removed: value.flags.contains(ThumbnailFlags::Removed),
            score: value.votes - value.downvotes,
            username: None,
            vip: false,
        }
    }
}
#[cfg(feature = "dearrow-parser")]
impl IntoWithDatabase<ApiThumbnail> for &dearrow_parser::Thumbnail {
    fn into_with_db(self, db: &dearrow_parser::DearrowDB) -> ApiThumbnail {
        let mut res: ApiThumbnail = self.into();
        res.username = db.usernames.get(&res.user_id).map(|u| u.username.clone());
        res.vip = db.vip_users.contains(&res.user_id);
        res
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct User {
    pub user_id: Arc<str>,
    pub username: Option<Arc<str>>,
    pub username_locked: bool,
    pub vip: bool,
    pub title_count: u64,
    pub thumbnail_count: u64,
}
