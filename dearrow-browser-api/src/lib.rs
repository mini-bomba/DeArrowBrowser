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

#[cfg(feature = "sync")]
pub mod sync {
    type RcStr = std::sync::Arc<str>;

    include!("api.rs");

    #[cfg(feature = "dearrow-parser")]
    pub use dearrow_parser_traits::*;

    #[cfg(feature = "dearrow-parser")]
    mod dearrow_parser_traits {
        use std::sync::{Arc, LazyLock};

        use super::*;
        use dearrow_parser::db::DearrowDB;
        use dearrow_parser::types::{self as parser_types};
        use strum::VariantNames;

        static CASUAL_CATEGORY_ARCS: LazyLock<&'static [Arc<str>]> = LazyLock::new(||
            Box::leak(
                parser_types::CasualCategory::VARIANTS.iter()
                    .map(|v| Arc::from(*v))
                    .collect()
            )
        );

        pub trait IntoWithDatabase<T> {
            fn into_with_db(self, db: &DearrowDB) -> T;
        }

        impl From<&parser_types::Title> for ApiTitle {
            fn from(value: &parser_types::Title) -> Self {
                use parser_types::TitleFlags;
                let unverified = value.flags.contains(TitleFlags::Unverified);
                Self {
                    uuid: value.uuid.clone(),
                    video_id: value.video_id.clone(),
                    title: value.title.clone(),
                    user_id: value.user_id.clone(),
                    user_agent: value.user_agent.clone(),
                    time_submitted: value.time_submitted,
                    votes: value.votes,
                    downvotes: value.downvotes,
                    original: value.flags.contains(TitleFlags::Original),
                    locked: value.flags.contains(TitleFlags::Locked),
                    shadow_hidden: value.flags.contains(TitleFlags::ShadowHidden),
                    unverified,
                    removed: value.flags.contains(TitleFlags::Removed),
                    casual_mode: value.flags.contains(TitleFlags::CasualMode),
                    votes_missing: value.flags.contains(TitleFlags::MissingVotes),
                    score: value.votes - value.downvotes - i8::from(unverified),
                    username: None,
                    vip: false,
                }
            }
        }
        impl IntoWithDatabase<ApiTitle> for &parser_types::Title {
            fn into_with_db(self, db: &DearrowDB) -> ApiTitle {
                let mut res: ApiTitle = self.into();
                res.username = db.get_username(&res.user_id).map(|u| u.username.clone());
                res.vip = db.is_vip(&res.user_id);
                res
            }
        }

        impl From<&parser_types::Thumbnail> for ApiThumbnail {
            fn from(value: &parser_types::Thumbnail) -> Self {
                use parser_types::ThumbnailFlags;
                Self {
                    uuid: value.uuid.clone(),
                    video_id: value.video_id.clone(),
                    user_id: value.user_id.clone(),
                    user_agent: value.user_agent.clone(),
                    time_submitted: value.time_submitted,
                    timestamp: value.timestamp,
                    votes: value.votes,
                    downvotes: value.downvotes,
                    original: value.flags.contains(ThumbnailFlags::Original),
                    locked: value.flags.contains(ThumbnailFlags::Locked),
                    shadow_hidden: value.flags.contains(ThumbnailFlags::ShadowHidden),
                    removed: value.flags.contains(ThumbnailFlags::Removed),
                    casual_mode: value.flags.contains(ThumbnailFlags::CasualMode),
                    votes_missing: value.flags.contains(ThumbnailFlags::MissingVotes),
                    timestamp_missing: value.flags.contains(ThumbnailFlags::MissingTimestamp),
                    score: value.votes - value.downvotes,
                    username: None,
                    vip: false,
                }
            }
        }
        impl IntoWithDatabase<ApiThumbnail> for &parser_types::Thumbnail {
            fn into_with_db(self, db: &DearrowDB) -> ApiThumbnail {
                let mut res: ApiThumbnail = self.into();
                res.username = db.get_username(&res.user_id).map(|u| u.username.clone());
                res.vip = db.is_vip(&res.user_id);
                res
            }
        }

        impl IntoWithDatabase<ApiWarning> for &parser_types::Warning {
            fn into_with_db(self, db: &DearrowDB) -> ApiWarning {
                let warned_username = db
                    .get_username(&self.warned_user_id)
                    .map(|u| u.username.clone());
                let issuer_username = db
                    .get_username(&self.issuer_user_id)
                    .map(|u| u.username.clone());
                ApiWarning {
                    warned_user_id: self.warned_user_id.clone(),
                    warned_username,
                    issuer_user_id: self.issuer_user_id.clone(),
                    issuer_username,
                    time_issued: self.time_issued,
                    time_acknowledged: self.time_acknowledged,
                    message: self.message.clone(),
                    active: self.active,
                    extension: match self.extension {
                        parser_types::Extension::SponsorBlock => Extension::SponsorBlock,
                        parser_types::Extension::DeArrow => Extension::DeArrow,
                    },
                }
            }
        }

        impl From<&parser_types::CasualTitle> for ApiCasualTitle {
            fn from(value: &parser_types::CasualTitle) -> Self {
                Self {
                    title: value.title.clone(),
                    video_id: value.video_id.clone(),
                    first_submitted: value.first_submitted,
                    votes: value.votes.into_iter()
                        .filter_map(|(c, v)| Some(c).zip(v))
                        .map(|(category, count)| (CASUAL_CATEGORY_ARCS[category as usize].clone(), count))
                        .collect(),
                }
            }
        }

        impl User {
            pub fn from_db(db: &DearrowDB, user_id: &Arc<str>, username: Option<&parser_types::Username>) -> User {
                let (warning_count, active_warnings) = db.warnings.iter().fold((0, 0), |acc, w| {
                    if !Arc::ptr_eq(&w.warned_user_id, user_id) {
                        acc
                    } else if w.active {
                        (acc.0 + 1, acc.1 + 1)
                    } else {
                        (acc.0 + 1, acc.1)
                    }
                });
                let mut last_title_submission = None;
                let mut last_thumb_submission = None;
                let mut title_submission_intervals = vec![];
                let mut thumb_submission_intervals = vec![];
                User {
                    user_id: user_id.clone(),
                    username: username.map(|u| u.username.clone()),
                    username_locked: username.is_some_and(|u| u.locked),
                    vip: db.vip_users.contains(user_id),
                    title_count: db
                        .titles
                        .iter()
                        .filter(|t| Arc::ptr_eq(&t.user_id, user_id))
                        .inspect(|t| {
                            if let Some(prev_time) = last_title_submission {
                                #[allow(clippy::cast_precision_loss)]
                                title_submission_intervals.push((t.time_submitted - prev_time) as f64);
                            }
                            last_title_submission = Some(t.time_submitted);
                        })
                        .count() as u64,
                    thumbnail_count: db
                        .thumbnails
                        .iter()
                        .filter(|t| Arc::ptr_eq(&t.user_id, user_id))
                        .inspect(|t| {
                            if let Some(prev_time) = last_thumb_submission {
                                #[allow(clippy::cast_precision_loss)]
                                thumb_submission_intervals.push((t.time_submitted - prev_time) as f64);
                            }
                            last_thumb_submission = Some(t.time_submitted);
                        })
                        .count() as u64,
                    warning_count,
                    active_warning_count: active_warnings,
                    last_submission: last_title_submission
                        .zip(last_thumb_submission)
                        .map(|(t1, t2)| t1.max(t2))
                        .or_else(|| last_title_submission.or(last_thumb_submission)),
                    title_submission_rate: StatisticalSummary::from_measurements(&mut title_submission_intervals),
                    thumbnail_submission_rate: StatisticalSummary::from_measurements(&mut thumb_submission_intervals),
                }
            }
        }
    }
}
#[cfg(feature = "unsync")]
pub mod unsync {
    type RcStr = std::rc::Rc<str>;

    include!("api.rs");
}
#[cfg(feature = "boxed")]
pub mod boxed {
    type RcStr = Box<str>;

    include!("api.rs");
}
#[cfg(feature = "string")]
pub mod string {
    type RcStr = String;

    include!("api.rs");
}
