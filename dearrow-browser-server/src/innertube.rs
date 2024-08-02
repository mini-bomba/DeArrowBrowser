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

use actix_web::{get, web, HttpResponse};
use anyhow::{anyhow, Context};
use dearrow_browser_api::sync::InnertubeVideo;
use reqwest::Client;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize};

use crate::utils;

type JsonResult<T> = utils::Result<web::Json<T>>;

pub fn configure_disabled(cfg: &mut web::ServiceConfig) {
    cfg.default_service(web::to(disabled_route));
}

pub fn configure_enabled(cfg: &mut web::ServiceConfig) {
    cfg.service(get_innertube_video);
}

async fn disabled_route() -> HttpResponse {
    HttpResponse::NotFound().body("Innertube endpoints are disabled on this DeArrow Browser instance.")
}


// https://github.com/ajayyy/DeArrow/blob/c4e1375380bc3b0cb202af283f0e7b4e5e6e30f1/src/thumbnails/thumbnailData.ts#L232
#[derive(Serialize, Clone)]
struct ITInput<'a> {
    context: ITInputContext,
    #[serde(rename="videoId")]
    video_id: &'a str
}

// https://github.com/ajayyy/DeArrow/blob/c4e1375380bc3b0cb202af283f0e7b4e5e6e30f1/src/thumbnails/thumbnailData.ts#L233
#[derive(Serialize, Clone, Default)]
struct ITInputContext {
    client: ITInputClient,
}

// https://github.com/ajayyy/DeArrow/blob/c4e1375380bc3b0cb202af283f0e7b4e5e6e30f1/src/thumbnails/thumbnailData.ts#L234
#[derive(Serialize, Clone)]
struct ITInputClient {
    #[serde(rename="clientName")]
    client_name: &'static str,
    #[serde(rename="clientVersion")]
    client_version: &'static str,
}

impl Default for ITInputClient {
    fn default() -> Self {
        Self {
            client_name: "WEB",
            client_version: "2.20230327.07.00",
        }
    }
}

#[derive(Deserialize)]
struct ITOutput {
    #[serde(rename="videoDetails")]
    video_details: ITOutputVideo,
}

#[derive(Deserialize)]
struct ITOutputVideo {
    #[serde(rename="videoId")]
    video_id: String,
    #[serde(rename="lengthSeconds", deserialize_with="deserialize_stringnum")]
    length_seconds: u64,
}

struct StringNumVisitor;

impl<'de> Visitor<'de> for StringNumVisitor {
    type Value = u64;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a u64 value or a string parseable into a u64 value")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where E: serde::de::Error, 
    {
        Ok(v)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where E: serde::de::Error, 
    {
        v.parse().map_err(serde::de::Error::custom)
    }
}

pub fn deserialize_stringnum<'de, D>(deserializer: D) -> Result<u64, D::Error>
where D: Deserializer<'de>
{
    deserializer.deserialize_any(StringNumVisitor)
}

// https://github.com/ajayyy/DeArrow/blob/c4e1375380bc3b0cb202af283f0e7b4e5e6e30f1/src/thumbnails/thumbnailData.ts#L230
#[get("/video/{video_id}")]
pub async fn get_innertube_video(path: web::Path<String>, client: web::Data<Client>) -> JsonResult<InnertubeVideo> {
    let vid = path.as_str();
    let url = reqwest::Url::parse("https://www.youtube.com/youtubei/v1/player").context("Failed to construct an innertube request URL")?;
    let input = ITInput {
        context: ITInputContext::default(),
        video_id: vid,
    };
    let resp = client.post(url).json(&input).send().await.context("Failed to send innertube request")?;
    let resp = resp.error_for_status().context("Innertube request failed")?;
    let result: ITOutput = resp.json().await.context("Failed to deserialize innertube response")?;
    if result.video_details.video_id != vid {
        return Err(anyhow!("Innertube returned the wrong videoid - requested: {vid}, got: {}", result.video_details.video_id).into());
    }
    Ok(web::Json(InnertubeVideo {
        video_id: vid.into(),
        duration: result.video_details.length_seconds,
    }))
}
