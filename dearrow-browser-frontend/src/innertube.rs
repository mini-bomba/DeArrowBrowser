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

use error_handling::{ErrorContext, ResContext};
use reqwest::Url;
use serde::Deserialize;

use crate::utils::{api_request, ReqwestUrlExt};


#[derive(Deserialize)]
pub struct OEmbedResponse {
    pub title: String,
    pub author_url: String,
}

pub async fn get_oembed_info(vid: &str) -> Result<OEmbedResponse, ErrorContext> {
    let mut url = YOUTUBE_OEMBED_URL.with(Clone::clone);
    url.query_pairs_mut()
        .clear()
        .append_pair("url", youtu_be_link(vid).as_str());
    api_request(url).await.context("oembed request failed")
}

pub fn youtu_be_link(vid: &str) -> Url {
    let mut url = YOUTU_BE_URL.with(Clone::clone);
    url.extend_segments(&[vid]).expect("https://youtu.be/ should be a valid base");
    url
}

pub fn original_thumbnail_url(vid: &str) -> Url {
    let mut url = THUMBNAIL_URL.with(Clone::clone);
    url.extend_segments(&[vid, "maxresdefault.jpg"]).expect("youtube thumbnail url should be a valid base");
    url
}

thread_local! {
    static YOUTU_BE_URL: Url = Url::parse("https://youtu.be/").expect("should be able to parse youtu.be base URL");
    static YOUTUBE_OEMBED_URL: Url = Url::parse("https://www.youtube-nocookie.com/oembed").expect("should be able to parse youtube-nocookie oembed URL");
    static THUMBNAIL_URL: Url = Url::parse("https://img.youtube.com/vi").expect("should be able to parse the youtube thumbnail URL");
}
