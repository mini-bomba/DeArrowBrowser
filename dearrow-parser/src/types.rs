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

use std::sync::Arc;

use enum_map::{Enum, EnumMap};
use enumflags2::{bitflags, BitFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use strum::VariantNames;

use crate::dedupe::{Dedupe, StringSet};

#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum ThumbnailFlags {
    Original,
    Locked,
    ShadowHidden,
    Removed,
    MissingVotes,
    MissingTimestamp,
}

#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TitleFlags {
    Original,
    Locked,
    ShadowHidden,
    Unverified,
    Removed,
    MissingVotes,
}

#[derive(Clone, Debug)]
pub struct Thumbnail {
    pub uuid: Arc<str>,
    pub video_id: Arc<str>,
    pub user_id: Arc<str>,
    pub time_submitted: i64,
    pub timestamp: Option<f64>,
    pub votes: i8,
    pub downvotes: i8,
    pub flags: BitFlags<ThumbnailFlags>,
    pub hash_prefix: u16,
}

#[derive(Clone, Debug)]
pub struct Title {
    pub uuid: Arc<str>,
    pub video_id: Arc<str>,
    pub title: Arc<str>,
    pub user_id: Arc<str>,
    pub time_submitted: i64,
    pub votes: i8,
    pub downvotes: i8,
    pub flags: BitFlags<TitleFlags>,
    pub hash_prefix: u16,
}

#[derive(Clone, Debug)]
pub struct Username {
    pub user_id: Arc<str>,
    pub username: Arc<str>,
    pub locked: bool,
}

/// All times in this struct are represented as fractions of the video duration
#[derive(Clone, Copy, Debug)]
pub struct UncutSegment {
    pub offset: f64,
    pub length: f64,
}

#[derive(Clone, Debug)]
pub struct VideoInfo {
    pub video_id: Arc<str>,
    pub video_duration: f64,
    /// Sorted slice of `UncutSegments`
    pub uncut_segments: Box<[UncutSegment]>,
    pub has_outro: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Extension {
    SponsorBlock,
    DeArrow,
}

#[derive(Clone, Debug)]
pub struct Warning {
    pub warned_user_id: Arc<str>,
    pub issuer_user_id: Arc<str>,
    pub time_issued: i64,
    pub extension: Extension,
    pub message: Arc<str>,
    pub active: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug, Enum, VariantNames)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum CasualCategory {
    Funny,
    Creative,
    Clever,
    Descriptive,
    Other,
    Downvote,
}

#[derive(Clone, Debug)]
pub struct CasualTitle {
    pub video_id: Arc<str>,
    pub hash_prefix: u16,
    pub title: Option<Arc<str>>,
    pub first_submitted: i64,
    pub votes: EnumMap<CasualCategory, Option<i16>>,
}

impl Dedupe for Thumbnail {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.uuid);
        set.dedupe_arc(&mut self.video_id);
        set.dedupe_arc(&mut self.user_id);
    }
}
impl Dedupe for Title {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.uuid);
        set.dedupe_arc(&mut self.title);
        set.dedupe_arc(&mut self.video_id);
        set.dedupe_arc(&mut self.user_id);
    }
}
impl Dedupe for Username {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.user_id);
        set.dedupe_arc(&mut self.username);
    }
}

impl Dedupe for Warning {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.warned_user_id);
        set.dedupe_arc(&mut self.issuer_user_id);
        set.dedupe_arc(&mut self.message);
    }
}

impl Dedupe for CasualTitle {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.video_id);
        if let Some(ref mut title) = self.title {
            set.dedupe_arc(title);
        }
    }
}

pub fn compute_hashprefix(s: &str) -> u16 {
    let mut hasher = Sha256::new();
    hasher.update(s);
    let hash = hasher.finalize();
    u16::from_be_bytes([hash[0], hash[1]])
}
