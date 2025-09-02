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

use std::rc::Rc;

use cloneable_errors::{ErrorContext, ResContext};
use gloo_console::error;
use yew::prelude::*;
use yew_router::prelude::Link;

use crate::components::icon::{Icon, IconType};
use crate::components::links::videoid_link;
use crate::constants::YOUTUBE_EMBED_URL;
use crate::innertube::{self, youtu_be_link};
use crate::pages::MainRoute;
use crate::utils_common::ReqwestUrlExt;

#[derive(Properties, PartialEq, Clone)]
pub struct YoutubeProps {
    pub videoid: AttrValue,
}

#[function_component]
pub fn YoutubeIframe(props: &YoutubeProps) -> Html {
    let embed_url: Rc<AttrValue> = use_memo(props.videoid.clone(), |vid| {
        let mut url = YOUTUBE_EMBED_URL.clone();
        url.extend_segments(&[vid]).unwrap();
        AttrValue::Rc(url.as_str().into())
    });

    html! {<iframe src={&*embed_url} allowfullscreen=true />}
}

#[derive(Properties, PartialEq, Clone)]
pub struct VideoLinkProps {
    pub videoid: AttrValue,
    pub multiline: bool,
}

#[function_component]
pub fn YoutubeVideoLink(props: &VideoLinkProps) -> Html {
    let youtube_url: Rc<AttrValue> = use_memo(props.videoid.clone(), |vid| AttrValue::Rc(youtu_be_link(vid).as_str().into()));
    html!{
        <>
            <a href={&*youtube_url} title="View this video on YouTube" target="_blank">{props.videoid.clone()}</a>
            if props.multiline {
                <br />
            } else {
                {" "}
            }
            {videoid_link(props.videoid.clone())}
        </>
    }
}

type SimpleLoadState = crate::utils_app::SimpleLoadState<AttrValue>;

pub struct OriginalTitle {
    title: SimpleLoadState,
    version: u8,
}

impl OriginalTitle {
    fn refresh(&mut self, ctx: &Context<Self>) {
        self.version = self.version.wrapping_add(1);
        self.title = SimpleLoadState::Loading;
        let version = self.version;
        let video_id = ctx.props().videoid.clone();
        ctx.link().send_future(async move {
            let result: Result<_, ErrorContext> = async {
                let result = innertube::get_oembed_info(&video_id).await.context("Failed to fetch oembed info")?;
                Ok(AttrValue::Rc(Rc::from(result.title)))
            }.await;
            let result = match result {
                Ok(handle) => SimpleLoadState::Ready(handle),
                Err(err) => {
                    error!(format!("Failed to fetch original title for video {video_id}: {err:?}"));
                    SimpleLoadState::Failed
                }
            };
            OembedMessage::Fetched { result, version }
        });
    }
}

pub struct ChannelLink {
    handle: SimpleLoadState,
    version: u8,
}

impl ChannelLink {
    fn refresh(&mut self, ctx: &Context<Self>) {
        self.version = self.version.wrapping_add(1);
        self.handle = SimpleLoadState::Loading;
        let version = self.version;
        let video_id = ctx.props().videoid.clone();
        ctx.link().send_future(async move {
            let result: Result<_, ErrorContext> = async {
                let result = innertube::get_oembed_info(&video_id).await.context("Failed to fetch oembed info")?;
                let url = reqwest::Url::parse(&result.author_url).context("Failed to parse channel URL")?;
                let handle = url
                    .path_segments()
                    .context("Failed to extract channel handle from URL: not a base???")?
                    .filter(|s| !s.is_empty())
                    .next_back()
                    .context("Failed to extract channel handle from URL: URL has no segments")?;
                Ok(AttrValue::Rc(Rc::from(handle)))
            }.await;
            let result = match result {
                Ok(handle) => SimpleLoadState::Ready(handle),
                Err(err) => {
                    error!(format!("Failed to fetch channel handle for video {video_id}: {err:?}"));
                    SimpleLoadState::Failed
                }
            };
            OembedMessage::Fetched { result, version }
        });
    }
}

pub enum OembedMessage {
    Fetched { result: SimpleLoadState, version: u8 },
}

impl Component for OriginalTitle {
    type Message = OembedMessage;
    type Properties = YoutubeProps;

    fn create(ctx: &Context<Self>) -> Self {
        let mut this = Self {
            title: SimpleLoadState::Loading,
            version: 0,
        };
        this.refresh(ctx);
        this
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        match &self.title {
            SimpleLoadState::Loading => html! {<em>{"Loading..."}</em>},
            SimpleLoadState::Failed => html! {<em>{"Unknown"}</em>},
            SimpleLoadState::Ready(title) => html! {<span>{title}</span>}
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            OembedMessage::Fetched { result, version } => {
                if self.version == version {
                    self.title = result;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if ctx.props().videoid == old_props.videoid {
            false
        } else {
            self.refresh(ctx);
            true
        }
    }
}

impl Component for ChannelLink {
    type Message = OembedMessage;
    type Properties = YoutubeProps;

    fn create(ctx: &Context<Self>) -> Self {
        let mut this = Self {
            handle: SimpleLoadState::Loading,
            version: 0,
        };
        this.refresh(ctx);
        this
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        match &self.handle {
            SimpleLoadState::Loading => html! {<em>{"Loading..."}</em>},
            SimpleLoadState::Failed => html! {<em>{"Unknown"}</em>},
            SimpleLoadState::Ready(handle) => {
                let route = MainRoute::Channel {
                    id: handle.clone(),
                };
                html! {
                    <Link<MainRoute> to={route}>{handle}<Icon r#type={IconType::DABLogo} /></Link<MainRoute>>
                }
            }
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            OembedMessage::Fetched { result, version } => {
                if self.version == version {
                    self.handle = result;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if ctx.props().videoid == old_props.videoid {
            false
        } else {
            self.refresh(ctx);
            true
        }
    }
}
