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

use enum_map::EnumMap;
use enumflags2::BitFlag;

use crate::{
    csv::types::*,
    dedupe::{Dedupe, StringSet},
    errors::{ObjectKind, ParseError, ParseErrorKind}, 
    types::{self, compute_hashprefix, ThumbnailFlags, TitleFlags}
};

type Result<T> = std::result::Result<T, ParseError>;
type ResultWithWarnings<T> = std::result::Result<WithWarnings<T>, ParseError>;


pub struct WithWarnings<T> {
    pub obj: T,
    pub warnings: Vec<ParseError>,
}

macro_rules! intbool {
    (thumb $struct:expr, $field:ident) => {
        intbool!(!$struct, $field, ObjectKind::Thumbnail, uuid, 0, 1, strict)
    };
    (title $struct:expr, $field:ident) => {
        intbool!(!$struct, $field, ObjectKind::Title, uuid, 0, 1, strict)
    };
    (uname $struct:expr, $field:ident) => {
        intbool!(!$struct, $field, ObjectKind::Username, user_id, 0, 1, strict)
    };
    (warn $struct:expr, $field:ident) => {
        intbool!(!$struct, $field, ObjectKind::Warning, user_id, 0, 1, strict)
    };
    (thumb $struct:expr, $field:ident, $falseint: expr, $trueint:expr, $forgiving:tt) => {
        intbool!(
            !$struct,
            $field,
            ObjectKind::Thumbnail,
            uuid,
            $falseint,
            $trueint,
            $forgiving
        )
    };
    (title $struct:expr, $field:ident, $falseint: expr, $trueint:expr, $forgiving:tt) => {
        intbool!(
            !$struct,
            $field,
            ObjectKind::Title,
            uuid,
            $falseint,
            $trueint,
            $forgiving
        )
    };
    (uname $struct:expr, $field:ident, $falseint: expr, $trueint:expr, $forgiving:tt) => {
        intbool!(
            !$struct,
            $field,
            ObjectKind::Username,
            user_id,
            $falseint,
            $trueint,
            $forgiving
        )
    };
    (warn $struct:expr, $field:ident, $falseint: expr, $trueint:expr, $forgiving:tt) => {
        intbool!(
            !$struct,
            $field,
            ObjectKind::Warning,
            user_id,
            $falseint,
            $trueint,
            $forgiving
        )
    };
    (! $struct:expr, $field:ident, $kind:expr, $uuid:ident, $falseint:expr, $trueint:expr, strict) => {
        match $struct.$field {
            $falseint => false,
            $trueint => true,
            value => {
                return Err(ParseError(
                    $kind,
                    ParseErrorKind::InvalidValue {
                        uuid: $struct.$uuid.clone(),
                        field: stringify!($field),
                        value,
                    },
                ))
            }
        }
    };
    (! $struct:expr, $field:ident, $kind:expr, $uuid:ident, $falseint:expr, $trueint:expr, forgiving) => {
        match $struct.$field {
            $falseint => false,
            $trueint => true,
            _ => true,
        }
    };
}

impl Thumbnail {
    pub fn try_merge(
        self,
        timestamps: Option<&ThumbnailTimestamps>,
        votes: Option<&ThumbnailVotes>,
    ) -> ResultWithWarnings<types::Thumbnail> {
        match &timestamps {
            Some(timestamp) if self.uuid != timestamp.uuid => {
                return Err(ParseError(
                    ObjectKind::Thumbnail,
                    ParseErrorKind::MismatchedUUIDs {
                        struct_name: "ThumbnailTimestamps",
                        uuid_main: self.uuid,
                        uuid_struct: timestamp.uuid.clone(),
                    },
                ));
            }
            _ => {}
        }
        match &votes {
            Some(votes) if self.uuid != votes.uuid => {
                return Err(ParseError(
                    ObjectKind::Thumbnail,
                    ParseErrorKind::MismatchedUUIDs {
                        struct_name: "ThumbnailVotes",
                        uuid_main: self.uuid,
                        uuid_struct: votes.uuid.clone(),
                    },
                ));
            }
            _ => {}
        }
        let mut warnings = Vec::new();
        let mut flags = ThumbnailFlags::empty();
        if votes.is_none() {
            warnings.push(ParseError(
                ObjectKind::Thumbnail,
                ParseErrorKind::MissingSubobject {
                    struct_name: "ThumbnailVotes",
                    uuid: self.uuid.clone(),
                },
            ));
            flags.set(ThumbnailFlags::MissingVotes, true);
        }
        let votes = votes.unwrap_or_default();
        flags.set(ThumbnailFlags::Original, intbool!(thumb self, original));
        flags.set(ThumbnailFlags::Locked, intbool!(thumb votes, locked));
        flags.set(
            ThumbnailFlags::ShadowHidden,
            intbool!(thumb votes, shadow_hidden, 0, 1, forgiving),
        );
        flags.set(ThumbnailFlags::Removed, intbool!(thumb votes, removed));
        flags.set(ThumbnailFlags::CasualMode, intbool!(thumb self, casual_mode));
        if !flags.contains(ThumbnailFlags::Original) && timestamps.is_none() {
            warnings.push(ParseError(
                ObjectKind::Thumbnail,
                ParseErrorKind::MissingSubobject {
                    struct_name: "ThumbnailTimestamps",
                    uuid: self.uuid.clone(),
                },
            ));
            flags.set(ThumbnailFlags::MissingTimestamp, true);
        }
        Ok(WithWarnings {
            obj: types::Thumbnail {
                uuid: self.uuid,
                user_id: self.user_id,
                user_agent: self.user_agent,
                time_submitted: self.time_submitted,
                timestamp: timestamps.map(|t| t.timestamp),
                votes: votes.votes,
                downvotes: votes.downvotes,
                flags,
                hash_prefix: match u16::from_str_radix(&self.hashed_video_id[..4], 16) {
                    Ok(n) => n,
                    Err(_) => compute_hashprefix(&self.video_id),
                },
                video_id: self.video_id,
            },
            warnings,
        })
    }
}

impl Title {
    pub fn try_merge(self, votes: Option<&TitleVotes>) -> ResultWithWarnings<types::Title> {
        match &votes {
            Some(votes) if self.uuid != votes.uuid => {
                return Err(ParseError(
                    ObjectKind::Title,
                    ParseErrorKind::MismatchedUUIDs {
                        struct_name: "TitleVotes",
                        uuid_main: self.uuid,
                        uuid_struct: votes.uuid.clone(),
                    },
                ));
            }
            _ => {}
        }
        let mut warnings = Vec::new();
        let mut flags = TitleFlags::empty();
        if votes.is_none() {
            warnings.push(ParseError(
                ObjectKind::Title,
                ParseErrorKind::MissingSubobject {
                    struct_name: "TitleVotes",
                    uuid: self.uuid.clone(),
                },
            ));
            flags.set(TitleFlags::MissingVotes, true);
        }
        let votes = votes.unwrap_or_default();
        flags.set(TitleFlags::Original, intbool!(title self, original));
        flags.set(TitleFlags::Locked, intbool!(title votes, locked));
        flags.set(
            TitleFlags::ShadowHidden,
            intbool!(title votes, shadow_hidden, 0, 1, forgiving),
        );
        flags.set(
            TitleFlags::Unverified,
            intbool!(title votes, verification, 0, -1, strict),
        );
        flags.set(TitleFlags::Removed, intbool!(title votes, removed));
        flags.set(TitleFlags::CasualMode, intbool!(title self, casual_mode));
        Ok(WithWarnings {
            obj: types::Title {
                uuid: self.uuid,
                title: self.title,
                user_id: self.user_id,
                user_agent: self.user_agent,
                time_submitted: self.time_submitted,
                votes: votes.votes,
                downvotes: votes.downvotes,
                flags,
                hash_prefix: match u16::from_str_radix(&self.hashed_video_id[..4], 16) {
                    Ok(n) => n,
                    Err(_) => compute_hashprefix(&self.video_id),
                },
                video_id: self.video_id,
            },
            warnings,
        })
    }
}

impl SponsorTime {
    pub fn filter_and_split(mut self, string_set: &mut StringSet) -> Option<(u16, VideoDuration, TrimmedSponsorTime)> {
        let hash_prefix = match u16::from_str_radix(&self.hashed_video_id[..4], 16) {
            Ok(n) => n,
            Err(_) => compute_hashprefix(&self.video_id),
        };
        // insert all userids, to register them as "seen" so that the usernames of these users
        // are kept, even if we don't actually use these userids
        string_set.dedupe_arc(&mut self.user_id);
        // https://github.com/ajayyy/SponsorBlockServer/blob/af31f511a53a7e30ad27123656a911393200672b/src/routes/getBranding.ts#L112
        if self.votes > -2
            && self.shadow_hidden == 0
            && self.hidden == 0
            && self.action_type == "skip"
        {
            self.dedupe(string_set);
            Some((
                hash_prefix,
                VideoDuration {
                    video_id: self.video_id.clone(),
                    video_duration: self.video_duration,
                    time_submitted: self.time_submitted,
                    has_outro: self.category == "outro",
                },
                TrimmedSponsorTime {
                    video_id: self.video_id,
                    start_time: self.start_time,
                    end_time: self.end_time,
                },
            ))
        } else {
            None
        }
    }
}

impl TryFrom<Username> for types::Username {
    type Error = ParseError;

    fn try_from(value: Username) -> Result<types::Username> {
        let locked = intbool!(uname value, locked);
        Ok(types::Username {
            user_id: value.user_id,
            username: value.username,
            locked,
        })
    }
}

impl TryFrom<Warning> for types::Warning {
    type Error = ParseError;

    fn try_from(value: Warning) -> Result<types::Warning> {
        let active = intbool!(uname value, enabled);
        let extension = match value.r#type {
            0 => types::Extension::SponsorBlock,
            1 => types::Extension::DeArrow,
            v => {
                return Err(ParseError(
                    ObjectKind::Warning,
                    ParseErrorKind::InvalidValue {
                        uuid: value.user_id,
                        field: "active",
                        value: v,
                    },
                ))
            }
        };
        Ok(types::Warning {
            warned_user_id: value.user_id,
            issuer_user_id: value.issuer_user_id,
            time_issued: value.issue_time,
            message: value.reason,
            active,
            extension,
        })
    }
}

impl From<CasualTitle> for types::CasualTitle {
    fn from(value: CasualTitle) -> Self {
        Self {
            hash_prefix: match u16::from_str_radix(&value.hashed_video_id[..4], 16) {
                Ok(n) => n,
                Err(_) => compute_hashprefix(&value.video_id),
            },
            video_id: value.video_id,
            title: Some(value.title),
            first_submitted: i64::MAX,
            votes: EnumMap::default(),
        }
    }
}

impl From<CasualVote> for types::CasualTitle {
    fn from(value: CasualVote) -> Self {
        Self {
            hash_prefix: match u16::from_str_radix(&value.hashed_video_id[..4], 16) {
                Ok(n) => n,
                Err(_) => compute_hashprefix(&value.video_id),
            },
            video_id: value.video_id.clone(),
            title: None,
            first_submitted: value.time_submitted,
            votes: {
                let mut votes = EnumMap::default();
                votes[value.category] = Some(value.upvotes);
                votes
            },
        }
    }
}

impl types::CasualTitle {
    pub(crate) fn add_vote(&mut self, vote: CasualVote) -> Result<()> {
        if self.votes[vote.category].is_some() {
            return Err(ParseError(
                ObjectKind::CasualTitle,
                ParseErrorKind::DuplicateVote {
                    video_id: vote.video_id,
                    title_id: vote.title_id,
                    category: vote.category,
                }
            ))
        }

        self.first_submitted = self.first_submitted.min(vote.time_submitted);
        self.votes[vote.category] = Some(vote.upvotes);

        Ok(())
    }
}
