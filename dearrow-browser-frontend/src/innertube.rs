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

use anyhow::Context;
use reqwest::Url;
use serde::Deserialize;

use crate::utils::{get_reqwest_client, ReqwestUrlExt};


#[derive(Deserialize)]
struct OEmbedResponse {
    title: Option<String>,
}

pub async fn get_original_title(vid: &str) -> Result<String, anyhow::Error> {
    let url = Url::parse_with_params(
        "https://www.youtube-nocookie.com/oembed", 
        &[("url", &youtu_be_link(vid))]
    ).context("Failed to construct an oembed request URL")?;
    let resp: OEmbedResponse = get_reqwest_client().get(url).send().await.context("Failed to send oembed request")?
        .json().await.context("Failed to deserialize oembed response")?;
    resp.title.context("oembed response contained no title")
}

pub fn youtu_be_link(vid: &str) -> Url {
    let mut url = Url::parse("https://youtu.be/").expect("should be able to parse youtu.be base URL");
    url.extend_segments(&[vid]).expect("https://youtu.be/ should be a valid base");
    url
}
