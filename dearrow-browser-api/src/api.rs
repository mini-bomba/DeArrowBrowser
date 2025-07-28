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
// NOTE: This file is used as a template for dearrow-browser-api::sync and ::unsync modules.
//       The RcStr type will be defined externally with the correct smart pointer variant for the
//       module.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(default)]
pub struct StatusResponse {
    // database stats
    pub titles: Option<usize>,
    pub thumbnails: Option<usize>,
    pub vip_users: Option<usize>,
    pub usernames: Option<usize>,
    pub warnings: Option<usize>,
    // dab internal stats
    pub errors: Option<usize>,
    pub string_count: Option<usize>,
    pub video_infos: Option<usize>,
    pub uncut_segments: Option<usize>,
    pub cached_channels: Option<usize>,
    pub fscached_channels: Option<usize>,
    // general server build data
    pub server_version: Option<RcStr>,
    pub server_git_hash: Option<RcStr>,
    pub server_git_dirty: Option<bool>,
    pub server_build_timestamp: Option<i64>,
    pub server_startup_timestamp: Option<i64>,
    pub server_brand: Option<RcStr>,
    pub server_url: Option<RcStr>,
    // stats for snapshot-based impls
    pub last_updated: Option<i64>,
    pub last_modified: Option<i64>,
    pub updating_now: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ApiTitle {
    pub uuid: RcStr,
    pub video_id: RcStr,
    pub title: RcStr,
    pub user_id: RcStr,
    pub user_agent: RcStr,
    pub time_submitted: i64,
    pub votes: i8,
    pub downvotes: i8,
    pub original: bool,
    pub locked: bool,
    pub shadow_hidden: bool,
    pub unverified: bool,
    pub removed: bool,
    pub casual_mode: bool,
    #[serde(default)]
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
    pub user_agent: RcStr,
    pub time_submitted: i64,
    pub timestamp: Option<f64>,
    pub votes: i8,
    pub downvotes: i8,
    pub original: bool,
    pub locked: bool,
    pub shadow_hidden: bool,
    pub removed: bool,
    pub casual_mode: bool,
    #[serde(default)]
    pub votes_missing: bool,
    #[serde(default)]
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
    pub warning_count: u64,
    pub active_warning_count: u64,
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct InnertubeChannel {
    pub ucid: RcStr,
    pub channel_name: RcStr,
    pub num_videos: u64,
    pub num_vods: u64,
    pub num_shorts: u64,
    pub num_releases: u64,
    pub total_videos: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Copy)]
pub struct ChannelFetchProgress {
    pub videos: BrowseProgress,
    pub vods: BrowseProgress,
    pub shorts: BrowseProgress,
    pub releases_tab: BrowseProgress,
    pub releases_home: BrowseProgress,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Copy)]
pub struct BrowseProgress {
    pub videos_fetched: u64,
    pub videos_in_fscache: u64,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Extension {
    SponsorBlock,
    DeArrow,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ApiWarning {
    pub warned_user_id: RcStr,
    pub warned_username: Option<RcStr>,
    pub issuer_user_id: RcStr,
    pub issuer_username: Option<RcStr>,
    pub time_issued: i64,
    pub extension: Extension,
    pub message: RcStr,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ApiCasualTitle {
    pub video_id: RcStr,
    pub title: Option<RcStr>,
    pub first_submitted: i64,
    pub votes: HashMap<RcStr, i16>,
}
