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

use crate::components::icon::*;
use crate::components::links::userid_link;
use crate::components::youtube::{OriginalTitle, YoutubeIframe, YoutubeVideoLink};
use crate::hooks::use_async_suspension;
use crate::thumbnails::components::{Thumbnail, ThumbnailCaption};
use crate::utils::{get_reqwest_client, html_length, render_datetime, RcEq, ReqwestResponseExt};
use crate::WindowContext;

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

fn flags_entry(html: Html) -> Html {
    if html_length(&html) == 0 {
        html! {{"No flags"}}
    } else {
        html
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
        resp.check_status().await?
            .json::<ApiTitle>().await.context("Failed to deserialize API response").map(Some)
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
                    <div>
                        if title.votes_missing {
                            {"Score: No data"}
                        } else {
                            {format!("Score: {} upvotes, {} downvotes; Final score: {}", title.votes, title.downvotes, title.score)}
                        }
                    </div>
                    <div>
                        {"Flags: "}
                        {flags_entry(html!{<>
                        if title.votes_missing {
                            <br /><Icon r#type={IconType::VotesMissing} />{" - Broken entry: Missing an entry in the votes table"}
                        }
                        if title.original {
                            <br /><Icon r#type={IconType::Original} />{" - Original title"}
                        }
                        if title.unverified {
                            <br /><Icon r#type={IconType::Unverified} />{" - Submitted by an unverified user (-1 score)"}
                        }
                        if title.locked {
                            <br /><Icon r#type={IconType::Locked} />{" - Locked by a VIP"}
                        }
                        if title.vip {
                            <br /><Icon r#type={IconType::VIP} />{" - Submitted by a VIP"}
                        }
                        </>})}
                    </div>
                    <div>
                        {"Visibility: "}
                        if title.votes_missing {
                            <Icon r#type={IconType::VotesMissing} />{" Effectively hidden due to missing the votes table entry"}
                        } else if title.removed {
                            <Icon r#type={IconType::Removed} />{" Removed by VIP"}
                        } else if title.shadow_hidden {
                            <Icon r#type={IconType::ShadowHidden} />{" Hidden by VIP using batch actions (shadowhidden)"}
                        } else if title.votes - title.downvotes < -1 {
                            <Icon r#type={IconType::Downvote} />{" Removed by downvotes"}
                        } else if title.votes < 0 {
                            <Icon r#type={IconType::Replaced} />{" Replaced by submitter"}
                        } else if !title.locked && title.score < 0 {
                            <Icon r#type={IconType::PartiallyHidden} />{" Partially hidden - Only visible in submission menus"}
                        } else {
                            <Icon r#type={IconType::Upvote} />{" Fully visible"}
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
        resp.check_status().await?
            .json::<ApiThumbnail>().await.context("Failed to deserialize API response").map(Some)
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
                        if thumbnail.timestamp_missing {
                            {" Custom thumbnail without a timestamp"}
                        } else if let Some(time) = thumbnail.timestamp {
                            {time}
                        } else {
                            {"Original thumbnail"}
                        }
                    </div>
                    <div>{"Original title: "}<Suspense fallback={inline_placeholder}><OriginalTitle videoid={thumbnail.video_id.clone()} /></Suspense></div>
                    <div>
                        if thumbnail.votes_missing {
                            {"Score: No data"}
                        } else {
                            {format!("Score: {} upvotes, {} downvotes; Final score: {}", thumbnail.votes, thumbnail.downvotes, thumbnail.score)}
                        }
                    </div>
                    <div>
                        {"Flags: "}
                        {flags_entry(html!{<>
                        if thumbnail.timestamp_missing {
                            <br /><Icon r#type={IconType::TimestampMissing} />{" - Broken entry: Missing a timestamp value"}
                        }
                        if thumbnail.votes_missing {
                            <br /><Icon r#type={IconType::VotesMissing} />{" - Broken entry: Missing an entry in the votes table"}
                        }
                        if thumbnail.original {
                            <br /><Icon r#type={IconType::Original} />{" - Original thumbnail"}
                        }
                        if thumbnail.locked {
                            <br /><Icon r#type={IconType::Locked} />{" - Locked by a VIP"}
                        }
                        if thumbnail.vip {
                            <br /><Icon r#type={IconType::VIP} />{" - Submitted by a VIP"}
                        }
                        </>})}
                    </div>
                    <div>
                        {"Visibility: "}
                        if thumbnail.votes_missing {
                            <Icon r#type={IconType::VotesMissing} />{" Effectively hidden due to missing the votes table entry"}
                        } else if thumbnail.removed {
                            <Icon r#type={IconType::Removed} />{" Removed by VIP"}
                        } else if thumbnail.shadow_hidden {
                            <Icon r#type={IconType::ShadowHidden} />{" Hidden by VIP using batch actions (shadowhidden)"}
                        } else if thumbnail.score < -1 {
                            <Icon r#type={IconType::Downvote} />{" Removed by downvotes"}
                        } else if thumbnail.original && !thumbnail.locked && thumbnail.score < 1 {
                            <Icon r#type={IconType::Downvote} />{" Original thumbnail with insufficient score to be shown (requires >= 1 or lock)"}
                        } else if thumbnail.score < 0 {
                            <Icon r#type={IconType::PartiallyHidden} />{" Partially hidden - Only visible in submission menus"}
                        } else {
                            <Icon r#type={IconType::Upvote} />{" Fully visible"}
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
                if !thumbnail.timestamp_missing {
                    <Thumbnail video_id={thumbnail.video_id.clone()} timestamp={thumbnail.timestamp} caption={(*caption).clone()} />
                }
                <YoutubeIframe videoid={thumbnail.video_id.clone()} />
            </div>
        },
    })
}
