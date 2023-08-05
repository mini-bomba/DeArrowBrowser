use anyhow::Context;
use chrono::{DateTime, Utc, NaiveDateTime};
use reqwest::Url;
use serde::Deserialize;

const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn render_datetime(dt: DateTime<Utc>) -> String 
{
    format!("{}", dt.format(TIME_FORMAT))
}
pub fn render_naive_datetime(dt: NaiveDateTime) -> String 
{
    format!("{}", dt.format(TIME_FORMAT))
}
pub fn render_datetime_with_delta(dt: DateTime<Utc>) -> String
{
    format!("{} UTC ({} minutes ago)", dt.format(TIME_FORMAT), (Utc::now()-dt).num_minutes())
}

#[derive(Deserialize)]
struct OEmbedResponse {
    title: Option<String>,
}

pub async fn get_original_title(vid: String) -> Result<String, anyhow::Error> {
    let url = Url::parse_with_params(
        "https://youtube.com/oembed", 
        &[("url", format!("https://youtu.be/{vid}"))]
    ).context("Failed to construct an oembed request URL")?;
    let resp: OEmbedResponse = reqwest::get(url).await.context("Failed to send oembed request")?
        .json().await.context("Failed to deserialize oembed response")?;
    resp.title.context("oembed response contained no title")
}
