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

use anyhow::Context;
use chrono::DateTime;
use dearrow_browser_api::unsync::{ApiThumbnail, ApiTitle};
use reqwest::StatusCode;
use yew::prelude::*;

use crate::{components::{links::userid_link, youtube::{OriginalTitle, YoutubeIframe, YoutubeVideoLink}}, hooks::use_async_suspension, thumbnails::components::{Thumbnail, ThumbnailCaption}, utils::{get_reqwest_client, render_datetime, RcEq}, WindowContext};

#[derive(Properties, PartialEq, Clone)]
pub struct UUIDPageProps {
    pub uuid: AttrValue,
}

#[function_component]
pub fn UUIDPage(props: &UUIDPageProps) -> Html {
    let placeholder = html! {
        <h3>{"Loading..."}</h3>
    };
    html! {
        <>
            <h2>{"Title "}{props.uuid.clone()}</h2>
            <Suspense fallback={placeholder.clone()}>
                <UUIDTitle ..props.clone() />
            </Suspense>
            <h2>{"Thumbnail "}{props.uuid.clone()}</h2>
            <Suspense fallback={placeholder}>
                <UUIDThumbnail ..props.clone() />
            </Suspense>
        </>
    }
}

#[function_component]
fn UUIDTitle(props: &UUIDPageProps) -> HtmlResult {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let title = use_async_suspension(|(wc, uuid)| async move {
        let url = wc.origin_join_segments(&["api", "titles", "uuid", &uuid]);
        let resp = get_reqwest_client().get(url).send().await.context("API request failed")?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        resp.error_for_status_ref().context("API request failed")?;
        resp.json::<ApiTitle>().await.context("Failed to deserialize API response").map(Some)
    }, (window_context, props.uuid.clone()))?;

    let inline_placeholder = html! {<span>{"Loading..."}</span>};

    Ok(match *title {
        Err(ref e) => html! {
            <center>
                <b>{"Failed to fetch the title from the API :/"}</b>
                <pre>{format!("{e:?}")}</pre>
            </center>
        },
        Ok(None) => html! {
            <h3>{"Title not found"}</h3>
        },
        Ok(Some(ref title)) => html! {
            <div class="page-details">
                <div class="info-table">
                    <div>{"Video ID: "}<YoutubeVideoLink videoid={title.video_id.clone()} multiline={false} /></div>
                    <div>{"Title: "}{title.title.clone()}</div>
                    <div>{"Original title: "}<Suspense fallback={inline_placeholder}><OriginalTitle videoid={title.video_id.clone()} /></Suspense></div>
                    <div>{format!("Score: {} upvotes, {} downvotes; Final score: {}", title.votes, title.downvotes, title.score)}</div>
                    <div>
                        {"Flags: "}
                        if title.original {
                            <br />{"♻️  - Original title"}
                        }
                        if title.unverified {
                            <br />{"❓ - Submitted by an unverified user (-1 score)"}
                        }
                        if title.locked {
                            <br />{"🔒 - Locked by a VIP"}
                        }
                        if title.vip {
                            <br />{"👑 - Submitted by a VIP"}
                        }
                        if !(title.original || title.unverified || title.locked || title.vip) {
                            {"No flags"}
                        }
                    </div>
                    <div>
                        {"Visibility: "}
                        if title.removed {
                            {"❌ Removed by VIP"}
                        } else if title.shadow_hidden {
                            {"🚫 Hidden by VIP using batch actions (shadowhidden)"}
                        } else if title.votes - title.downvotes < -1 {
                            {"👎 Removed by downvotes"}
                        } else if title.votes < 0 {
                            {"🔄 Replaced by submitter"}
                        } else if title.score < 0 {
                            <span class="grayscale">{"👎"}</span>{" Partially hidden - Only visible in submission menus"}
                        } else {
                            {"👍 Fully visible"}
                        }
                    </div>
                    <div>{"Submitted at: "}{DateTime::from_timestamp_millis(title.time_submitted).map_or(title.time_submitted.to_string(), render_datetime)}</div>
                    <div>{"User ID: "}{title.user_id.clone()}{" "}{userid_link(title.user_id.clone().into())}</div>
                    <div>
                        {"Username: "}
                        if let Some(ref username) = title.username {
                            {username.clone()}
                        } else {
                            <em>{"No username set"}</em>
                        }
                    </div>
                </div>
                <YoutubeIframe videoid={title.video_id.clone()} />
            </div>
        },
    })
}

#[function_component]
fn UUIDThumbnail(props: &UUIDPageProps) -> HtmlResult {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let thumbnail = use_async_suspension(|(wc, uuid)| async move {
        let url = wc.origin_join_segments(&["api", "thumbnails", "uuid", &uuid]);
        let resp = get_reqwest_client().get(url).send().await.context("API request failed")?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        resp.error_for_status_ref().context("API request failed")?;
        resp.json::<ApiThumbnail>().await.context("Failed to deserialize API response").map(Some)
    }, (window_context, props.uuid.clone()))?;
    let caption: Rc<ThumbnailCaption> = use_memo(RcEq(thumbnail.clone()), |thumbnail| {
        if let Ok(Some(ref thumbnail)) = **thumbnail {
            if let Some(timestamp) = thumbnail.timestamp {
                ThumbnailCaption::Text(format!("{} @ {}", thumbnail.video_id, timestamp).into())
            } else {
                ThumbnailCaption::Text(format!("Original thumbnail of {}", thumbnail.video_id).into())
            }
        } else {
            ThumbnailCaption::None
        }
    });

    let inline_placeholder = html! {<span>{"Loading..."}</span>};

    Ok(match *thumbnail {
        Err(ref e) => html! {
            <center>
                <b>{"Failed to fetch the thumbnail from the API :/"}</b>
                <pre>{format!("{e:?}")}</pre>
            </center>
        },
        Ok(None) => html! {
            <h3>{"Thumbnail not found"}</h3>
        },
        Ok(Some(ref thumbnail)) => html! {
            <div class="page-details">
                <div class="info-table">
                    <div>{"Video ID: "}<YoutubeVideoLink videoid={thumbnail.video_id.clone()} multiline={false} /></div>
                    <div>{"Timestamp: "}
                        if let Some(time) = thumbnail.timestamp {
                            {time}
                        } else {
                            {"Original thumbnail"}
                        }
                    </div>
                    <div>{"Original title: "}<Suspense fallback={inline_placeholder}><OriginalTitle videoid={thumbnail.video_id.clone()} /></Suspense></div>
                    <div>{format!("Score: {} upvotes, {} downvotes; Final score: {}", thumbnail.votes, thumbnail.downvotes, thumbnail.score)}</div>
                    <div>
                        {"Flags: "}
                        if thumbnail.original {
                            <br />{"♻️  - Original thumbnail"}
                        }
                        if thumbnail.locked {
                            <br />{"🔒 - Locked by a VIP"}
                        }
                        if thumbnail.vip {
                            <br />{"👑 - Submitted by a VIP"}
                        }
                        if !(thumbnail.original || thumbnail.locked || thumbnail.vip) {
                            {"No flags"}
                        }
                    </div>
                    <div>
                        {"Visibility: "}
                        if thumbnail.removed {
                            {"❌ Removed by VIP"}
                        } else if thumbnail.shadow_hidden {
                            {"🚫 Hidden by VIP using batch actions (shadowhidden)"}
                        } else if thumbnail.votes - thumbnail.downvotes < -1 {
                            {"👎 Removed by downvotes"}
                        } else if thumbnail.score < 0 {
                            <span class="grayscale">{"👎"}</span>{" Partially hidden - Only visible in submission menus"}
                        } else {
                            {"👍 Fully visible"}
                        }
                    </div>
                    <div>{"Submitted at: "}{DateTime::from_timestamp_millis(thumbnail.time_submitted).map_or(thumbnail.time_submitted.to_string(), render_datetime)}</div>
                    <div>{"User ID: "}{thumbnail.user_id.clone()}{" "}{userid_link(thumbnail.user_id.clone().into())}</div>
                    <div>
                        {"Username: "}
                        if let Some(ref username) = thumbnail.username {
                            {username.clone()}
                        } else {
                            <em>{"No username set"}</em>
                        }
                    </div>
                </div>
                <Thumbnail video_id={thumbnail.video_id.clone()} timestamp={thumbnail.timestamp} caption={(*caption).clone()} />
                <YoutubeIframe videoid={thumbnail.video_id.clone()} />
            </div>
        },
    })
}
