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

use std::{fmt::Display, sync::Arc};

use strum::VariantNames;

use crate::types::CasualCategory;

#[derive(Debug, Clone)]
pub enum ParseErrorKind {
    InvalidValue {
        uuid: Arc<str>,
        field: &'static str,
        value: i8,
    },
    MismatchedUUIDs {
        struct_name: &'static str,
        uuid_main: Arc<str>,
        uuid_struct: Arc<str>,
    },
    MissingSubobject {
        struct_name: &'static str,
        uuid: Arc<str>,
    },
    DuplicateVote {
        video_id: Arc<str>,
        title_id: i8,
        category: CasualCategory,
    },
    CasualTitleWithoutVotes {
        video_id: Arc<str>,
        title: Option<Arc<str>>,
    }
}

#[derive(Debug, Clone, Copy, strum::Display)]
pub enum ObjectKind {
    Title,
    Thumbnail,
    Username,
    Warning,
    CasualTitle,
}

#[derive(Debug, Clone)]
pub struct ParseError(pub ObjectKind, pub ParseErrorKind);

impl std::error::Error for ParseError {}
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let object_kind = &self.0;
        match self.1 {
            ParseErrorKind::InvalidValue { ref uuid, field, value } => write!(f, "Parsing error: Field {field} in {object_kind} {uuid} contained an invalid value: {value}"),
            ParseErrorKind::MismatchedUUIDs { struct_name, ref uuid_main, ref uuid_struct } => write!(f, "Merge error: Component {struct_name} of {object_kind} {uuid_main} had a different UUID: {uuid_struct}"),
            ParseErrorKind::MissingSubobject { struct_name, ref uuid } => write!(f, "Parsing error: {object_kind} {uuid} was missing an associated {struct_name} object"),
            ParseErrorKind::DuplicateVote { ref video_id, title_id, category } => write!(f, "Merge error: Casual title #{title_id} on video {video_id} has multiple vote entries for category {}", CasualCategory::VARIANTS[category as usize]),
            ParseErrorKind::CasualTitleWithoutVotes { ref video_id, title: Some(ref title) } => write!(f, "Merge error: Casual title \"{title}\" on video {video_id} had no associated CasualVote entries"),
            ParseErrorKind::CasualTitleWithoutVotes { ref video_id, title: None } => write!(f, "Merge error: Casual null title on video {video_id} had no associated CasualVote entries"),
        }
    }
}
