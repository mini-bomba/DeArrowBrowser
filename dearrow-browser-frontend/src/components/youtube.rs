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
use std::rc::Rc;

use gloo_console::error;
use reqwest::Url;
use yew::prelude::*;

use crate::components::links::videoid_link;
use crate::hooks::use_async_suspension;
use crate::innertube::{self, youtu_be_link};
use crate::utils::ReqwestUrlExt;

#[derive(Properties, PartialEq, Clone)]
pub struct YoutubeProps {
    pub videoid: AttrValue,
}

#[function_component]
pub fn YoutubeIframe(props: &YoutubeProps) -> Html {
    let embed_url: Rc<AttrValue> = use_memo(props.videoid.clone(), |vid| {
        let mut url = YOUTUBE_EMBED_URL.with(Clone::clone);
        url.extend_segments(&[vid]).unwrap();
        return AttrValue::Rc(url.as_str().into())
    });

    html! {<iframe src={&*embed_url} allowfullscreen=true />}
}

#[function_component]
pub fn OriginalTitle(props: &YoutubeProps) -> HtmlResult {
    let title = use_async_suspension(|vid| async move {
        let result = innertube::get_oembed_info(&vid).await;
        if let Err(ref e) = result {
            error!(format!("Failed to fetch original title for video {vid}: {e:?}"));
        }
        result.map(|r| r.title)
    }, props.videoid.clone())?;
    if let Ok(ref t) = *title {
        Ok(html!{<span>{t.as_str()}</span>})
    } else {
        Ok(html!{<span><em>{"Failed to fetch original title"}</em></span>})
    }
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

thread_local! {
    static YOUTUBE_EMBED_URL: Url = Url::parse("https://www.youtube-nocookie.com/embed/").expect("should be able to parse the youtube embed url");
}
