/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024-2025 mini_bomba
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
use std::{sync::LazyLock, time::Duration};

use chrono::{DateTime, FixedOffset};
use regex::Regex;
use reqwest::{Url, Client};

use crate::built_info;

pub static REQWEST_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);
pub const ASYNC_TASK_AUTO_DISMISS_DELAY: Duration = Duration::from_secs(15);

// Data based on build-time constants

pub static VERSION_STRING: LazyLock<&'static str>                  = LazyLock::new(create_version_string);
pub static USER_AGENT:     LazyLock<&'static str>                  = LazyLock::new(create_useragent_string);
pub static COMMIT_LINK:    LazyLock<&'static str>                  = LazyLock::new(create_commit_link);
pub static BUILD_TIME:     LazyLock<Option<DateTime<FixedOffset>>> = LazyLock::new(|| DateTime::parse_from_rfc2822(built_info::BUILT_TIME_UTC).ok());
pub static COMMIT_TIME:    LazyLock<Option<DateTime<FixedOffset>>> = LazyLock::new(|| built_info::GIT_COMMIT_TIMESTAMP.and_then(|t| DateTime::parse_from_rfc3339(t).ok()));

// URLs

pub static YOUTU_BE_URL:       LazyLock<Url> = LazyLock::new(|| Url::parse("https://youtu.be/").expect("should be able to parse youtu.be base URL"));
pub static YOUTUBE_OEMBED_URL: LazyLock<Url> = LazyLock::new(|| Url::parse("https://www.youtube-nocookie.com/oembed").expect("should be able to parse youtube-nocookie oembed URL"));
pub static YOUTUBE_EMBED_URL:  LazyLock<Url> = LazyLock::new(|| Url::parse("https://www.youtube-nocookie.com/embed/").expect("should be able to parse the youtube embed url"));
pub static THUMBNAIL_URL:      LazyLock<Url> = LazyLock::new(|| Url::parse("https://img.youtube.com/vi").expect("should be able to parse the youtube thumbnail URL"));
pub static SBB_BASE:           LazyLock<Url> = LazyLock::new(|| Url::parse("https://sb.ltn.fi/").expect("should be able to parse sb.ltn.fi base URL"));
pub const SBS_BRANDING_ENDPOINT: &[&str]     = &["api", "branding"];

// Regexes

pub static UUID_REGEX:     LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9a-f]{8}\-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$").expect("UUID_REGEX should be valid"));
pub static SHA256_REGEX:   LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9a-f]{64}$").expect("SHA256_REGEX should be valid"));
pub static UCID_REGEX:     LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^UC(?-u:[\w-]){22}$").expect("UCID_REGEX should be valid"));
pub static HANDLE_REGEX:   LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^@[\w.-]{3,30}$").expect("HANDLE_REGEX should be valid"));
pub static VIDEO_ID_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\w\d_-]{11}$").expect("VIDEO_ID_REGEX should be valid"));

fn create_version_string() -> &'static str {
    match (crate::built_info::GIT_COMMIT_HASH_SHORT, crate::built_info::GIT_DIRTY) {
        (Some(hash), Some(true)) => format!("{}+g{hash}-dirty", crate::built_info::PKG_VERSION).leak(),
        (Some(hash), _) => format!("{}+g{hash}", crate::built_info::PKG_VERSION).leak(),
        _ => crate::built_info::PKG_VERSION,
    }
}

fn create_useragent_string() -> &'static str {
    format!("DeArrowBrowser/{}", *VERSION_STRING).leak()
}

fn create_commit_link() -> &'static str {
    format!("https://github.com/mini-bomba/DeArrowBrowser/commit/{}", built_info::GIT_COMMIT_HASH.unwrap_or("")).leak()
}
