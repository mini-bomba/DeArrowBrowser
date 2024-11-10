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

use std::rc::Rc;

use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct PostBrandingBody {
    #[serde(rename="videoID")]
    pub video_id: Rc<str>,
    #[serde(rename="userID")]
    pub user_id: Rc<str>,
    pub user_agent: &'static str,
    pub service: &'static str,
    #[serde(skip_serializing_if="Option::is_none")]
    pub title: Option<SBServerTitle>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub thumbnail: Option<SBServerThumbnail>,
    pub downvote: bool,
    pub auto_lock: bool,
}

#[derive(Clone, Serialize)]
pub struct SBServerTitle {
    pub title: Rc<str>
}

#[derive(Clone, Serialize)]
pub struct SBServerThumbnail {
    #[serde(skip_serializing_if="Option::is_none")]
    pub timestamp: Option<f64>,
    pub original: bool,
}
