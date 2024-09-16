/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024 mini_bomba
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
use std::{sync::{Arc, LazyLock}, time::Duration};
use chrono::DateTime;
use error_handling::{ErrorContext, anyhow};
use regex::Regex;
use actix_web::http::StatusCode;

use crate::{built_info, innertube::BrowseMode};

pub static NUMBER_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d+").expect("Should be able to parse the number regex"));

// Paths
pub const CONFIG_PATH: &str = "config.toml";
pub const FSCACHE_TEMPDIR: &str = "tmp";
pub const FSCACHE_PLAYLISTS: &str = "playlists";

// Limits
pub static IT_TIMEOUT: Duration = Duration::from_secs(1);
pub static FSCACHE_SIZE_CACHE_DURATION: Duration = Duration::from_secs(60);

// Locking errors
pub static SS_READ_ERR:  LazyLock<ErrorContext> = LazyLock::new(|| anyhow!("Failed to acquire StringSet for reading"));
pub static SS_WRITE_ERR: LazyLock<ErrorContext> = LazyLock::new(|| anyhow!("Failed to acquire StringSet for writing"));
pub static DB_READ_ERR:  LazyLock<ErrorContext> = LazyLock::new(|| anyhow!("Failed to acquire DatabaseState for reading"));
pub static DB_WRITE_ERR: LazyLock<ErrorContext> = LazyLock::new(|| anyhow!("Failed to acquire DatabaseState for writing"));

// Innertube API urls
pub static IT_PLAYER_URL: LazyLock<reqwest::Url> = LazyLock::new(|| reqwest::Url::parse("https://www.youtube.com/youtubei/v1/player").expect("Should be able to parse the IT_PLAYER_URL"));
pub static IT_BROWSE_URL: LazyLock<reqwest::Url> = LazyLock::new(|| reqwest::Url::parse("https://www.youtube.com/youtubei/v1/browse").expect("Should be able to parse the IT_BROWSE_URL"));
pub static YT_BASE_URL:   LazyLock<reqwest::Url> = LazyLock::new(|| reqwest::Url::parse("https://www.youtube.com/").expect("Should be able to parse the YT_BASE_URL"));

// Innertube API browse modes
pub const IT_BROWSE_VIDEOS:   BrowseMode = BrowseMode { param: "EgZ2aWRlb3PyBgQKAjoA",         tab_name: "Videos",   cache_dir: "channels/videos" };
pub const IT_BROWSE_LIVE:     BrowseMode = BrowseMode { param: "EgdzdHJlYW1z8gYECgJ6AA%3D%3D", tab_name: "Live",     cache_dir: "channels/vods" };
pub const IT_BROWSE_SHORTS:   BrowseMode = BrowseMode { param: "EgZzaG9ydHPyBgUKA5oBAA%3D%3D", tab_name: "Shorts",   cache_dir: "channels/shorts" };
pub const IT_BROWSE_RELEASES: BrowseMode = BrowseMode { param: "EghyZWxlYXNlc_IGBQoDsgEA",     tab_name: "Releases", cache_dir: "" };
pub const IT_BROWSE_HOME:     BrowseMode = BrowseMode { param: "",                             tab_name: "Home",     cache_dir: "" };
pub const IT_RELEASES_SHELF_NAME: &str = "Albums & Singles";

// Youtube channel IDs and handles
// https://stackoverflow.com/a/16326307
pub static UCID_EXTRACTION_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"externalId":"([^"]+)""#).expect("Should be able to parse the UCID extraction regex"));
// https://github.com/yt-dlp/yt-dlp/blob/a065086640e888e8d58c615d52ed2f4f4e4c9d18/yt_dlp/extractor/youtube.py#L518-L519
pub static UCID_REGEX:        LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^UC(?-u:[\w-]){22}$").expect("Should be able to parse the UCID regex"));
pub static HANDLE_REGEX:      LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^@[\w.-]{3,30}$").expect("Should be able to parse the @handle regex"));

// Parsed built_info fields
pub static SERVER_VERSION:  LazyLock<Arc<str>> = LazyLock::new(|| built_info::PKG_VERSION.into());
pub static SERVER_GIT_HASH: LazyLock<Option<Arc<str>>> = LazyLock::new(|| built_info::GIT_COMMIT_HASH.map(std::convert::Into::into));
pub static BUILD_TIMESTAMP: LazyLock<Option<i64>> = LazyLock::new(|| DateTime::parse_from_rfc2822(built_info::BUILT_TIME_UTC).ok().map(|t| t.timestamp()));

// Custom response status codes
/// 333 Not ready yet - Indicates that the server is still querying requested data.
/// The response may contain progress information. The client should request the same URL again.
pub static NOT_READY_YET: LazyLock<StatusCode> = LazyLock::new(|| StatusCode::from_u16(333).expect("333 should be a 'valid' status code"));
