use reqwest::Url;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::hooks::use_navigator;

use crate::pages::MainRoute;

macro_rules! search_block {
    ($id:expr, $name:expr, $callback:expr) => {
        html! {
            <div>
                <label for={$id} >{concat!("Search by ", $name)}</label>
                <input id={$id} placeholder={$name} onkeydown={$callback} value="" />
            </div>
        }
    };
}

#[function_component]
pub fn Searchbar() -> Html {
    let navigator = use_navigator().expect("navigator should exist");

    let uuid_search = {
        let navigator = navigator.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                navigator.push(&MainRoute::NotImplemented);
            }
        })
    };
    let uid_search = {
        let navigator = navigator.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                let input: HtmlInputElement = e.target_unchecked_into();
                navigator.push(&MainRoute::User {id: input.value()});
            }
        })
    };
    let vid_search = { 
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                let input: HtmlInputElement = e.target_unchecked_into();
                let value = input.value();
                navigator.push(&MainRoute::Video {
                    id: if let Ok(url) = Url::parse(&value) {  // Try to parse as URL
                        url.query_pairs().find(|(ref k, _)| k == "v").map(|(_, v)| v.to_string()).or_else(||  // Try to find a "v" query param
                            url.path_segments().and_then(|it| it.filter(|s| !s.is_empty()).last()).map(ToString::to_string)  // Fall back to last non-empty path segment if none found
                        ).unwrap_or(value)  // Fall back to original value
                    } else {
                        value  // Fall back to original value
                    }
                });
            }
        })
    };

    html! {
        <div id="searchbar">
            {search_block!("uuid_search", "UUID", uuid_search)}
            {search_block!("vid_search", "Video ID", vid_search)}
            {search_block!("uid_search", "User ID", uid_search)}
        </div>
    }
}
