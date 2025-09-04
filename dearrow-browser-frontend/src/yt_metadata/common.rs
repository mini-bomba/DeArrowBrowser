/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2025 mini_bomba
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

use bincode::{Decode, Encode};
use reqwest::Url;

use crate::{constants::YOUTU_BE_URL, utils_common::ReqwestUrlExt};


#[derive(Debug, Decode, Encode, Clone)]
pub struct VideoMetadata {
    pub title: Rc<str>,
    pub channel: Rc<str>,
}

#[derive(Debug, Decode, Encode, Clone, Default)]
pub struct MetadataCacheStats {
    pub total: usize,
    pub pending: usize,
    pub cached: usize,
    pub failed: usize,
}

pub fn youtu_be_link(vid: &str) -> Url {
    let mut url = YOUTU_BE_URL.clone();
    url.extend_segments(&[vid]).expect("https://youtu.be/ should be a valid base");
    url
}
