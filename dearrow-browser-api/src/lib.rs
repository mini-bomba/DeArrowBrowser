use std::sync::Arc;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct StatusResponse {
    pub last_updated: i64,
    pub last_modified: i64,
    pub updating_now: bool,
    pub titles: usize,
    pub thumbnails: usize,
    pub vip_users: usize,
    pub usernames: usize,
    pub errors: usize,
    pub last_error: Option<String>,
    pub string_count: Option<usize>,
}

pub type ErrorList = Vec<String>;

#[cfg(feature = "dearrow-parser")]
pub trait IntoWithDatabase<T> {
    fn into_with_db(self, db: &dearrow_parser::DearrowDB) -> T;
}

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
            original: value.flags.contains(TitleFlags::Original),
            locked: value.flags.contains(TitleFlags::Locked),
            shadow_hidden: value.flags.contains(TitleFlags::ShadowHidden),
            unverified,
            score: if unverified {
                value.votes - 1
            } else {
                value.votes
            },
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
            original: value.flags.contains(ThumbnailFlags::Original),
            locked: value.flags.contains(ThumbnailFlags::Locked),
            shadow_hidden: value.flags.contains(ThumbnailFlags::ShadowHidden),
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

