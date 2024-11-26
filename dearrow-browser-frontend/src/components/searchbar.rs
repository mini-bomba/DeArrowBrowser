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

use reqwest::Url;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::{hooks::use_navigator, prelude::Link};

use crate::constants::{HANDLE_REGEX, SHA256_REGEX, UCID_REGEX, UUID_REGEX, VIDEO_ID_REGEX};
use crate::pages::MainRoute;
use crate::contexts::SettingsContext;

macro_rules! search_block {
    ($id:expr, $name:expr, $keydown_callback:expr, $input_callback:expr) => {
        html! {
            <div>
                <label for={$id} >{concat!("Search by ", $name)}</label>
                <input id={$id} placeholder={$name} onkeydown={$keydown_callback} oninput={$input_callback} value="" />
            </div>
        }
    };
}

fn parsed_url_last_segment(url: &Url) -> Option<String> {
    url.path_segments()
       .and_then(|it| 
            it.filter(|s| !s.is_empty())
              .last()
       )
       .map(ToString::to_string)
}

fn last_url_segment(value: &str) -> Option<String> {
    Url::parse(value)
        .ok()
        .as_ref()
        .and_then(parsed_url_last_segment)
}

fn url_query_or_last_segment(value: &str, query_key: &str) -> Option<String> {
    Url::parse(value)
        .ok()
        .and_then(|url|
            url.query_pairs()
               .find(|(ref k, _)| k == query_key)
               .map(|(_, v)| v.to_string())
               .or_else(|| parsed_url_last_segment(&url))
        )
}

#[function_component]
pub fn Searchbar() -> Html {
    let navigator = use_navigator().expect("navigator should exist");
    let settings_ctx: SettingsContext = use_context().expect("settings context should be available");
    let settings = settings_ctx.settings();
    let autosearch = settings.enable_autosearch;

    let uuid_search = {
        let navigator = navigator.clone();
        use_callback((), move |e: KeyboardEvent, ()| {
            if e.key() != "Enter" { return; }
            let input: HtmlInputElement = e.target_unchecked_into();
            let value = input.value().trim().to_owned();

            navigator.push(&MainRoute::UUID { id: value.into() });
        })
    };
    let uuid_paste = {
        let navigator = navigator.clone();
        use_callback(autosearch, move |e: InputEvent, autosearch| {
            if !autosearch { return; }
            if e.input_type() != "insertFromPaste" { return; }
            let Some(data) = e.data() else { return; };

            let data = data.trim();
            let data = last_url_segment(data).unwrap_or_else(|| data.to_owned());

            if UUID_REGEX.is_match(&data) {
                navigator.push(&MainRoute::UUID { id: data.into() });
            }
        })
    };
    let uid_search = {
        let navigator = navigator.clone();
        use_callback((), move |e: KeyboardEvent, ()| {
            if e.key() != "Enter" { return; }
            let input: HtmlInputElement = e.target_unchecked_into();
            let value = input.value();
            let value = value.trim();

            navigator.push(&MainRoute::User {
                id: last_url_segment(value).unwrap_or_else(|| value.to_owned()).into()
            });
        })
    };
    let uid_paste = {
        let navigator = navigator.clone();
        use_callback(autosearch, move |e: InputEvent, autosearch| {
            if !autosearch { return; }
            if e.input_type() != "insertFromPaste" { return; }
            let Some(data) = e.data() else { return; };

            let data = data.trim();
            let data = last_url_segment(data).unwrap_or_else(|| data.to_owned());

            if SHA256_REGEX.is_match(&data) {
                navigator.push(&MainRoute::User { id: data.into() });
            }
        })
    };
    let vid_search = { 
        let navigator = navigator.clone();
        use_callback((), move |e: KeyboardEvent, ()| {
            if e.key() != "Enter" { return; }
            let input: HtmlInputElement = e.target_unchecked_into();
            let value = input.value();
            let value = value.trim();

            navigator.push(&MainRoute::Video {
                id: url_query_or_last_segment(value, "v").unwrap_or_else(|| value.to_owned()).into()
            });
        })
    };
    let vid_paste = {
        let navigator = navigator.clone();
        use_callback(autosearch, move |e: InputEvent, autosearch| {
            if !autosearch { return; }
            if e.input_type() != "insertFromPaste" { return; }
            let Some(data) = e.data() else { return; };

            let data = data.trim();
            let data = url_query_or_last_segment(data, "v").unwrap_or_else(|| data.to_owned());

            if VIDEO_ID_REGEX.is_match(&data) {
                navigator.push(&MainRoute::Video { id: data.into() });
            }
        })
    };
    let channel_search = { 
        let navigator = navigator.clone();
        use_callback((), move |e: KeyboardEvent, ()| {
            if e.key() != "Enter" { return; }
            let input: HtmlInputElement = e.target_unchecked_into();
            let value = input.value();
            let value = value.trim();

            navigator.push(&MainRoute::Channel {
                id: last_url_segment(value).unwrap_or_else(|| value.to_owned()).into()
            });
        })
    };
    let channel_paste = {
        use_callback(autosearch, move |e: InputEvent, autosearch| {
            if !autosearch { return; }
            if e.input_type() != "insertFromPaste" { return; }
            let Some(data) = e.data() else { return; };

            let data = data.trim();
            let data = last_url_segment(data).unwrap_or_else(|| data.to_owned());

            if HANDLE_REGEX.is_match(&data) || UCID_REGEX.is_match(&data) {
                navigator.push(&MainRoute::Channel { id: data.into() });
            }
        })
    };

    html! {
        <div id="searchbar">
            {search_block!("uuid_search", "UUID", uuid_search, uuid_paste)}
            {search_block!("vid_search", "Video ID", vid_search, vid_paste)}
            {search_block!("uid_search", "User ID", uid_search, uid_paste)}
            {search_block!("channel_search", "Channel", channel_search, channel_paste)}
            <fieldset>
                <legend>{"Filtered views"}</legend>
                <ul>
                    <li><Link<MainRoute> to={MainRoute::Unverified}>{"Unverified titles"}</Link<MainRoute>></li>
                    <li><Link<MainRoute> to={MainRoute::Broken}>{"Broken entries"}</Link<MainRoute>></li>
                </ul>
            </fieldset>
        </div>
    }
}
