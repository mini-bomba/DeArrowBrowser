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

use yew::prelude::*;

use crate::{components::{detail_table::*, youtube::{OriginalTitle, YoutubeIframe}}, contexts::WindowContext, hooks::use_location_state, utils::youtu_be_link};

#[derive(Properties, PartialEq)]
struct VideoDetailsTableProps {
    videoid: AttrValue,
    mode: DetailType,
}

#[function_component]
fn VideoDetailsTable(props: &VideoDetailsTableProps) -> Html {
    let youtube_url: Rc<AttrValue> = use_memo(props.videoid.clone(), |vid| AttrValue::Rc(youtu_be_link(vid).as_str().into()));
    let fallback = html!{
        <span><em>{"Loading..."}</em></span>
    };
    html! {
        <div class="info-table">
            <div>{format!("Video ID: {}", props.videoid)}</div>
            <div hidden={props.mode != DetailType::Title}>
                {"Original title: "}
                <Suspense {fallback}><OriginalTitle videoid={props.videoid.clone()} /></Suspense>
            </div>
            <div><a href={&*youtube_url}>{"View on YouTube"}</a></div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct VideoPageProps {
    pub videoid: AttrValue,
}

#[function_component]
pub fn VideoPage(props: &VideoPageProps) -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let state = use_location_state().get_state();
    let entry_count = use_state_eq(|| None);

    let api_url = use_memo((state.detail_table_mode, props.videoid.clone()), |(dtm, vid)|{
        match dtm {
            DetailType::Title => window_context.origin_join_segments(&["api", "titles", "video_id", vid]),
            DetailType::Thumbnail => window_context.origin_join_segments(&["api", "thumbnails", "video_id", vid]),
        }
    });

    let fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };
    
    html! {
        <>
            <div class="page-details">
                <YoutubeIframe videoid={props.videoid.clone()} />
                <VideoDetailsTable videoid={props.videoid.clone()} mode={state.detail_table_mode} />
            </div>
            <TableModeSwitch entry_count={*entry_count} />
            <Suspense {fallback}>
                <PaginatedDetailTableRenderer mode={state.detail_table_mode} url={api_url} {entry_count} hide_videoid=true />
            </Suspense>
        </>
    }
}
