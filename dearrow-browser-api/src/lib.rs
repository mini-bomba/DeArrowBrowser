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
    pub trait IntoWithDatabase<T> {
        fn into_with_db(self, db: &dearrow_parser::DearrowDB) -> T;
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
    #[cfg(feature = "dearrow-parser")]
    impl IntoWithDatabase<ApiTitle> for &dearrow_parser::Title {
        fn into_with_db(self, db: &dearrow_parser::DearrowDB) -> ApiTitle {
            let mut res: ApiTitle = self.into();
            res.username = db.usernames.get(&res.user_id).map(|u| u.username.clone());
            res.vip = db.vip_users.contains(&res.user_id);
            res
        }
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
    #[cfg(feature = "dearrow-parser")]
    impl IntoWithDatabase<ApiThumbnail> for &dearrow_parser::Thumbnail {
        fn into_with_db(self, db: &dearrow_parser::DearrowDB) -> ApiThumbnail {
            let mut res: ApiThumbnail = self.into();
            res.username = db.usernames.get(&res.user_id).map(|u| u.username.clone());
            res.vip = db.vip_users.contains(&res.user_id);
            res
        }
    }
    #[cfg(feature = "dearrow-parser")]
    impl From<&dearrow_parser::Warning> for ApiWarning {
        fn from(value: &dearrow_parser::Warning) -> Self {
            Self {
                warned_user_id: value.warned_user_id.clone(),
                issuer_user_id: value.issuer_user_id.clone(),
                time_issued: value.time_issued,
                message: value.message.clone(),
                active: value.active,
                extension: match value.extension {
                    dearrow_parser::Extension::SponsorBlock => Extension::SponsorBlock,
                    dearrow_parser::Extension::DeArrow => Extension::DeArrow,
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
