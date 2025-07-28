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

use chrono::DateTime;
use dearrow_browser_api::unsync::{ApiThumbnail, ApiTitle};
use cloneable_errors::{ErrorContext, ResContext};
use reqwest::{StatusCode, Url};
use yew::prelude::*;

use crate::components::icon::*;
use crate::components::links::userid_link;
use crate::components::youtube::{OriginalTitle, YoutubeIframe, YoutubeVideoLink};
use crate::constants::REQWEST_CLIENT;
use crate::thumbnails::components::{Thumbnail, ThumbnailCaption};
use crate::utils::{html_length, render_datetime, RcEq, ReqwestResponseExt, ReqwestUrlExt};
use crate::WindowContext;

#[derive(Properties, PartialEq, Clone)]
pub struct UUIDPageProps {
    pub uuid: AttrValue,
}

#[derive(Clone)]
pub enum DetailStatus<T> {
    Fetching,
    NotFound,
    Ready(Rc<T>),
    Failed(ErrorContext),
}

impl<T> DetailStatus<T> {
    pub fn is_fetching(&self) -> bool {
        matches!(self, DetailStatus::Fetching)
    }

    pub fn is_none(&self) -> bool {
        matches!(self, DetailStatus::NotFound)
    }
}

pub struct UUIDPage {
    title: DetailStatus<ApiTitle>,
    thumb: DetailStatus<ApiThumbnail>,
    origin: Url,
    version_idx: u8,

    _wc_handle: ContextHandle<Rc<WindowContext>>,
}

impl UUIDPage {
    fn fetch_details(ctx: &Context<Self>, origin: &Url, version: u8) {
        let uuid = &ctx.props().uuid;
        let scope = ctx.link();
        {
            let url = origin.join_segments(&["api", "titles", "uuid", uuid]).expect("origin should be a valid base");
            scope.send_future(async move {
                let res = async {
                    let resp = REQWEST_CLIENT.get(url).send().await.context("API request failed")?;
                    if resp.status() == StatusCode::NOT_FOUND {
                        return Ok(DetailStatus::NotFound);
                    }
                    resp.check_status()
                        .await?
                        .json::<ApiTitle>()
                        .await
                        .context("Failed to deserialize API response")
                        .map(Rc::new)
                        .map(DetailStatus::Ready)
                }.await;
                match res {
                    Ok(s) => UUIDPageMessage::TitleFetched(s, version),
                    Err(e) => UUIDPageMessage::TitleFetched(DetailStatus::Failed(e), version),
                }
            });
        }
        {
            let url = origin.join_segments(&["api", "thumbnails", "uuid", uuid]).expect("origin should be a valid base");
            scope.send_future(async move {
                let res = async {
                    let resp = REQWEST_CLIENT.get(url).send().await.context("API request failed")?;
                    if resp.status() == StatusCode::NOT_FOUND {
                        return Ok(DetailStatus::NotFound);
                    }
                    resp.check_status()
                        .await?
                        .json::<ApiThumbnail>()
                        .await
                        .context("Failed to deserialize API response")
                        .map(Rc::new)
                        .map(DetailStatus::Ready)
                }.await;
                match res {
                    Ok(s) => UUIDPageMessage::ThumbFetched(s, version),
                    Err(e) => UUIDPageMessage::ThumbFetched(DetailStatus::Failed(e), version),
                }
            });
        }
    }

    fn refresh(&mut self, ctx: &Context<Self>) {
        self.title = DetailStatus::Fetching;
        self.thumb = DetailStatus::Fetching;
        self.version_idx = self.version_idx.wrapping_add(1);
        Self::fetch_details(ctx, &self.origin, self.version_idx);
    }
}

pub enum UUIDPageMessage {
    WindowContextUpdated(Rc<WindowContext>),
    TitleFetched(DetailStatus<ApiTitle>, u8),
    ThumbFetched(DetailStatus<ApiThumbnail>, u8),
}

impl Component for UUIDPage {
    type Properties = UUIDPageProps;
    type Message = UUIDPageMessage;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();

        let (wc, wc_handle) = scope.context(scope.callback(UUIDPageMessage::WindowContextUpdated)).expect("WindowContext should be available");
        Self::fetch_details(ctx, &wc.origin, 0);

        UUIDPage {
            title: DetailStatus::Fetching,
            thumb: DetailStatus::Fetching,
            origin: wc.origin.clone(),
            version_idx: 0,

            _wc_handle: wc_handle
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let uuid = &ctx.props().uuid;
        html! {
            <>
            if self.title.is_fetching() || self.thumb.is_fetching() {
                <center><b>{"Loading..."}</b></center>
            }
            if self.title.is_none() && self.thumb.is_none() {
                <center><b>{"Detail with UUID "}{uuid}{" does not exist."}</b></center>
            }
            {match self.title {
                DetailStatus::Fetching | DetailStatus::NotFound => html!{},
                DetailStatus::Ready(ref title) => html! {
                    <>
                    <h2>{"Title "}{uuid}</h2>
                    <UUIDTitle title={title} />
                    </>
                },
                DetailStatus::Failed(ref error) => html! {
                    <>
                    <h2>{"Title "}{uuid}</h2>
                    <b>{"Failed to fetch the title from the API :/"}</b>
                    <pre>{format!("{error:?}")}</pre>
                    </>
                },
            }}
            {match self.thumb {
                DetailStatus::Fetching | DetailStatus::NotFound => html!{},
                DetailStatus::Ready(ref thumb) => html! {
                    <>
                    <h2>{"Thumbnail "}{uuid}</h2>
                    <UUIDThumbnail thumbnail={thumb} />
                    </>
                },
                DetailStatus::Failed(ref error) => html! {
                    <>
                    <h2>{"Thumbnail "}{uuid}</h2>
                    <b>{"Failed to fetch the thumbnail from the API :/"}</b>
                    <pre>{format!("{error:?}")}</pre>
                    </>
                },
            }}
            </>
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if ctx.props().uuid == old_props.uuid {
            return false;
        }
        self.refresh(ctx);
        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            UUIDPageMessage::WindowContextUpdated(wc) => {
                if wc.origin == self.origin { return false }
                // origin changed, do a refetch & rerender
                self.origin = wc.origin.clone();
                self.refresh(ctx);
                true
            }
            UUIDPageMessage::TitleFetched(title, version) => {
                // check version
                if version != self.version_idx { return false }
                // update, rerender
                self.title = title;
                true
            }
            UUIDPageMessage::ThumbFetched(thumb, version) => {
                // check version
                if version != self.version_idx { return false }
                // update, rerender
                self.thumb = thumb;
                true
            }
        }
    }
}

fn flags_entry(html: Html) -> Html {
    if html_length(&html) == 0 {
        html! {{"No flags"}}
    } else {
        html
    }
}

fn user_agent_icon(user_agent: &str) -> Html {
    let Some((first_part, ..)) = user_agent.split_once('/') else {
        return html! {
            <Icon r#type={IconType::Unknown} tooltip="Unknown user agent" />
        }
    };
    match first_part {
        "deArrow@ajay.app" => html! {<Icon r#type={IconType::Firefox} tooltip="Firefox addon (stable version)" />},
        "deArrowBETA@ajay.app" => html! {<Icon r#type={IconType::FirefoxDev} tooltip="Firefox addon (beta version)" />},
        "enamippconapkdmgfgjchkhakpfinmaj" => html! {<Icon r#type={IconType::Chromium} tooltip="Chromium extension (from the Chrome Web Store)" />},
        "dearrow-cli" => html! {<Icon r#type={IconType::DeArrowCLI} tooltip="DeArrow CLI" />},
        "app.ajay.dearrow.extension (2PCQH7P6MB)" => html! {<Icon r#type={IconType::Safari} tooltip="Safari extension (from the appstore)" />},
        "app.ajay.dearrow.DeArrow-for-YouTube.Extension (UNSIGNED)" => html! {<Icon r#type={IconType::Safari} tooltip="Safari extension (self-compiled)" />},
        "com.github.libretube" => html! {<Icon r#type={IconType::LibreTube} tooltip="LibreTube" />},
        _ => html! {<Icon r#type={IconType::Unknown} tooltip="Unknown user agent" />},
    }
}

#[derive(Properties, PartialEq, Clone)]
struct UUIDTitleProps {
    title: Rc<ApiTitle>,
}

#[function_component]
fn UUIDTitle(props: &UUIDTitleProps) -> Html {
    let title = &props.title;
    let inline_placeholder = html! {<span>{"Loading..."}</span>};

    html! {
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
                    if title.casual_mode {
                        <br /><Icon r#type={IconType::Casual} />{" - Submitted by a casual mode user"}
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
                <div class="useragent-row">
                    {"User agent: "}
                    if title.user_agent.is_empty() {
                        <em>{"Unknown"}</em>
                    } else {
                        {title.user_agent.clone()}{user_agent_icon(&title.user_agent)}
                    }
                </div>
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
    }
}

#[derive(Properties, PartialEq, Clone)]
struct UUIDThumbnailProps {
    thumbnail: Rc<ApiThumbnail>,
}

#[function_component]
fn UUIDThumbnail(props: &UUIDThumbnailProps) -> Html {
    let thumbnail = &props.thumbnail;
    let caption: Rc<ThumbnailCaption> = use_memo(RcEq(thumbnail.clone()), |thumb| {
        match (&thumb.video_id, thumb.timestamp) {
            (vid, Some(ts)) => ThumbnailCaption::Text(format!("{vid} @ {ts}").into()),
            (vid, None) => ThumbnailCaption::Text(format!("Original thumbnail of {vid}").into()),
        }
    });

    let inline_placeholder = html! {<span>{"Loading..."}</span>};

    html! {
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
                    if thumbnail.casual_mode {
                        <br /><Icon r#type={IconType::Casual} />{" - Submitted by a casual mode user"}
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
                <div class="useragent-row">
                    {"User agent: "}
                    if thumbnail.user_agent.is_empty() {
                        <em>{"Unknown"}</em>
                    } else {
                        {thumbnail.user_agent.clone()}{user_agent_icon(&thumbnail.user_agent)}
                    }
                </div>
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
    }
}
