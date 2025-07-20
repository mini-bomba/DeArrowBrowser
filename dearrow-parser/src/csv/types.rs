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

use std::sync::{Arc, LazyLock};

use serde::Deserialize;

use crate::dedupe::{Dedupe, StringSet};
use crate::types::CasualCategory;

#[derive(Deserialize)]
pub struct Thumbnail {
    #[serde(rename = "videoID")]
    pub video_id: Arc<str>,
    pub original: i8,
    #[serde(rename = "userID")]
    pub user_id: Arc<str>,
    #[serde(rename = "timeSubmitted")]
    pub time_submitted: i64,
    #[serde(rename = "UUID")]
    pub uuid: Arc<str>,
    #[serde(rename = "hashedVideoID")]
    pub hashed_video_id: String,
    #[serde(rename = "casualMode")]
    pub casual_mode: i8,
    #[serde(rename = "userAgent")]
    pub user_agent: Arc<str>,
}

#[derive(Deserialize)]
pub struct ThumbnailTimestamps {
    #[serde(rename = "UUID")]
    pub uuid: Arc<str>,
    pub timestamp: f64,
}

#[derive(Deserialize, Default)]
pub struct ThumbnailVotes {
    #[serde(rename = "UUID")]
    pub uuid: Arc<str>,
    pub votes: i8,
    pub locked: i8,
    #[serde(rename = "shadowHidden")]
    pub shadow_hidden: i8,
    pub downvotes: i8,
    pub removed: i8,
}

static DEFAULT_THUMBNAIL_VOTES: LazyLock<&'static ThumbnailVotes> =
    LazyLock::new(|| Box::leak(Box::new(ThumbnailVotes::default())));

impl Default for &ThumbnailVotes {
    fn default() -> Self {
        &DEFAULT_THUMBNAIL_VOTES
    }
}

#[derive(Deserialize)]
pub struct Title {
    #[serde(rename = "videoID")]
    pub video_id: Arc<str>,
    pub title: Arc<str>,
    pub original: i8,
    #[serde(rename = "userID")]
    pub user_id: Arc<str>,
    #[serde(rename = "timeSubmitted")]
    pub time_submitted: i64,
    #[serde(rename = "UUID")]
    pub uuid: Arc<str>,
    #[serde(rename = "hashedVideoID")]
    pub hashed_video_id: String,
    #[serde(rename = "casualMode")]
    pub casual_mode: i8,
    #[serde(rename = "userAgent")]
    pub user_agent: Arc<str>,
}

#[derive(Deserialize, Default)]
pub struct TitleVotes {
    #[serde(rename = "UUID")]
    pub uuid: Arc<str>,
    pub votes: i8,
    pub locked: i8,
    #[serde(rename = "shadowHidden")]
    pub shadow_hidden: i8,
    pub verification: i8,
    pub downvotes: i8,
    pub removed: i8,
}

static DEFAULT_TITLE_VOTES: LazyLock<&'static TitleVotes> =
    LazyLock::new(|| Box::leak(Box::new(TitleVotes::default())));

impl Default for &TitleVotes {
    fn default() -> Self {
        &DEFAULT_TITLE_VOTES
    }
}

#[derive(Deserialize)]
pub struct VIPUser {
    #[serde(rename = "userID")]
    pub user_id: Arc<str>,
}

#[derive(Deserialize)]
pub struct Username {
    #[serde(rename = "userID")]
    pub user_id: Arc<str>,
    #[serde(rename = "userName")]
    pub username: Arc<str>,
    pub locked: i8,
}

#[derive(Deserialize)]
pub struct SponsorTime {
    #[serde(rename = "videoID")]
    pub video_id: Arc<str>,
    #[serde(rename = "startTime")]
    pub start_time: f64,
    #[serde(rename = "endTime")]
    pub end_time: f64,
    #[serde(rename = "videoDuration")]
    pub video_duration: f64,
    pub votes: i16,
    #[serde(rename = "shadowHidden")]
    pub shadow_hidden: i8,
    pub hidden: i8,
    pub category: String,
    #[serde(rename = "actionType")]
    pub action_type: String,
    #[serde(rename = "hashedVideoID")]
    pub hashed_video_id: String,
    #[serde(rename = "timeSubmitted")]
    pub time_submitted: i64,
    #[serde(rename = "userID")]
    pub user_id: Arc<str>,
}

#[derive(Deserialize)]
pub struct Warning {
    #[serde(rename = "userID")]
    pub user_id: Arc<str>,
    #[serde(rename = "issueTime")]
    pub issue_time: i64,
    #[serde(rename = "issuerUserID")]
    pub issuer_user_id: Arc<str>,
    pub enabled: i8,
    pub reason: Arc<str>,
    pub r#type: i8,
}

pub struct TrimmedSponsorTime {
    pub video_id: Arc<str>,
    pub start_time: f64,
    pub end_time: f64,
}

#[derive(Clone)]
pub struct VideoDuration {
    pub video_id: Arc<str>,
    pub time_submitted: i64,
    pub video_duration: f64,
    pub has_outro: bool,
}

#[derive(Deserialize)]
pub struct CasualTitle {
    #[serde(rename = "videoID")]
    pub video_id: Arc<str>,
    pub id: i8,
    #[serde(rename = "hashedVideoID")]
    pub hashed_video_id: String,
    pub title: Arc<str>,
}

#[derive(Deserialize)]
pub struct CasualVote {
    #[serde(rename = "videoID")]
    pub video_id: Arc<str>,
    #[serde(rename = "hashedVideoID")]
    pub hashed_video_id: String,
    pub category: CasualCategory,
    pub upvotes: i16,
    #[serde(rename = "timeSubmitted")]
    pub time_submitted: i64,
    #[serde(rename = "titleID")]
    pub title_id: i8,
}

impl Dedupe for Thumbnail {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.uuid);
        set.dedupe_arc(&mut self.video_id);
        set.dedupe_arc(&mut self.user_id);
        set.dedupe_arc(&mut self.user_agent);
    }
}
impl Dedupe for Title {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.uuid);
        set.dedupe_arc(&mut self.title);
        set.dedupe_arc(&mut self.video_id);
        set.dedupe_arc(&mut self.user_id);
        set.dedupe_arc(&mut self.user_agent);
    }
}
impl Dedupe for ThumbnailVotes {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.uuid);
    }
}
impl Dedupe for ThumbnailTimestamps {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.uuid);
    }
}
impl Dedupe for TitleVotes {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.uuid);
    }
}
impl Dedupe for VIPUser {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.user_id);
    }
}
impl Dedupe for Username {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.user_id);
        set.dedupe_arc(&mut self.username);
    }
}

impl Dedupe for SponsorTime {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.video_id);
    }
}

impl Dedupe for TrimmedSponsorTime {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.video_id);
    }
}

impl Dedupe for VideoDuration {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.video_id);
    }
}

impl Dedupe for Warning {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.user_id);
        set.dedupe_arc(&mut self.issuer_user_id);
        set.dedupe_arc(&mut self.reason);
    }
}

impl Dedupe for CasualTitle {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.video_id);
        set.dedupe_arc(&mut self.title);
    }
}

impl Dedupe for CasualVote {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.video_id);
    }
}
