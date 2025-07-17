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
#[cfg(feature = "sync")]
pub mod sync {
    type RcStr = std::sync::Arc<str>;

    include!("api.rs");

    #[cfg(feature = "dearrow-parser")]
    pub use dearrow_parser_traits::*;

    #[cfg(feature = "dearrow-parser")]
    mod dearrow_parser_traits {
        use super::*;
        use dearrow_parser::db::DearrowDB;
        use dearrow_parser::types as parser_types;

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
                    time_submitted: value.time_submitted,
                    votes: value.votes,
                    downvotes: value.downvotes,
                    original: value.flags.contains(TitleFlags::Original),
                    locked: value.flags.contains(TitleFlags::Locked),
                    shadow_hidden: value.flags.contains(TitleFlags::ShadowHidden),
                    unverified,
                    removed: value.flags.contains(TitleFlags::Removed),
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
                    time_submitted: value.time_submitted,
                    timestamp: value.timestamp,
                    votes: value.votes,
                    downvotes: value.downvotes,
                    original: value.flags.contains(ThumbnailFlags::Original),
                    locked: value.flags.contains(ThumbnailFlags::Locked),
                    shadow_hidden: value.flags.contains(ThumbnailFlags::ShadowHidden),
                    removed: value.flags.contains(ThumbnailFlags::Removed),
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
                    message: self.message.clone(),
                    active: self.active,
                    extension: match self.extension {
                        parser_types::Extension::SponsorBlock => Extension::SponsorBlock,
                        parser_types::Extension::DeArrow => Extension::DeArrow,
                    },
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
