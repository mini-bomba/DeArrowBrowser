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

use dearrow_browser_api::unsync::InnertubeChannel;
use yew::prelude::*;

use crate::components::detail_table::*;
use crate::contexts::{StatusContext, WindowContext};
use crate::hooks::{use_async_suspension, use_location_state};
use crate::utils::api_request;

#[function_component]
fn ChannelDetails(props: &ChannelPageProps) -> HtmlResult {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let status: StatusContext = use_context().expect("StatusResponse should be defined");
    let result: Rc<Result<InnertubeChannel, anyhow::Error>> = use_async_suspension(|(channel, _)| async move {
        let url = window_context.origin_join_segments(&["innertube","channel", &channel]);
        api_request(url).await
    }, (props.channel.clone(), status.map(|s| s.last_updated)))?;

    Ok(match *result {
        Ok(ref channel) => html! {
            <>
                <div>{format!("Channel name: {}", channel.channel_name)}</div>
                <div>{format!("Total videos: {}", channel.total_videos)}</div>
            </>
        },
        Err(ref e) => html! {
            <>
                <div>{"Failed to fetch channel data"}<br/><pre>{format!("{e:?}")}</pre></div>
            </>
        }
    })
}

#[derive(Properties, PartialEq, Clone)]
pub struct ChannelPageProps {
    pub channel: AttrValue,
}

#[function_component]
pub fn ChannelPage(props: &ChannelPageProps) -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let state = use_location_state().get_state();
    let entry_count = use_state_eq(|| None);

    let url = use_memo((state.detail_table_mode, props.channel.clone()), |(dtm, channel)| match dtm {
        DetailType::Title => window_context.origin_join_segments(&["api", "titles", "channel", channel]),
        DetailType::Thumbnail => window_context.origin_join_segments(&["api", "thumbnails", "channel", channel]),
    });

    let details_fallback = html! {
        <div><b>{"Loading..."}</b></div>
    };
    let table_fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };
    
    html! {
        <>
            <div class="page-details">
                <div class="info-table">
                    <Suspense fallback={details_fallback}><ChannelDetails ..{props.clone()} /></Suspense>
                </div>
            </div>
            <TableModeSwitch entry_count={*entry_count} />
            <Suspense fallback={table_fallback}>
                <PaginatedDetailTableRenderer mode={state.detail_table_mode} {url} {entry_count} />
            </Suspense>
        </>
    }
}
