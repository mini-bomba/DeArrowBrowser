/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2024 mini_bomba
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
// NOTE: This file is used as a template for dearrow-browser-api::sync and ::unsync modules.
//       The RcStr type will be defined externally with the correct smart pointer variant for the
//       module.
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
    pub server_version: RcStr,
    pub server_git_hash: Option<RcStr>,
    pub server_git_dirty: Option<bool>,
    pub server_build_timestamp: Option<i64>,
    pub server_startup_timestamp: i64,
}

pub type ErrorList = Vec<String>;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ApiTitle {
    pub uuid: RcStr,
    pub video_id: RcStr,
    pub title: RcStr,
    pub user_id: RcStr,
    pub time_submitted: i64,
    pub votes: i8,
    pub downvotes: i8,
    pub original: bool,
    pub locked: bool,
    pub shadow_hidden: bool,
    pub unverified: bool,
    pub removed: bool,
    pub votes_missing: bool,
    pub score: i8,
    pub username: Option<RcStr>,
    pub vip: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ApiThumbnail {
    pub uuid: RcStr,
    pub video_id: RcStr,
    pub user_id: RcStr,
    pub time_submitted: i64,
    pub timestamp: Option<f64>,
    pub votes: i8,
    pub downvotes: i8,
    pub original: bool,
    pub locked: bool,
    pub shadow_hidden: bool,
    pub removed: bool,
    pub votes_missing: bool,
    pub timestamp_missing: bool,
    pub score: i8,
    pub username: Option<RcStr>,
    pub vip: bool,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct User {
    pub user_id: RcStr,
    pub username: Option<RcStr>,
    pub username_locked: bool,
    pub vip: bool,
    pub title_count: u64,
    pub thumbnail_count: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Video {
    pub video_id: RcStr,
    pub random_thumbnail: f64,
    pub duration: Option<f64>,
    pub fraction_unmarked: f64,
    pub has_outro: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct InnertubeVideo {
    pub video_id: RcStr,
    pub duration: u64,
}
