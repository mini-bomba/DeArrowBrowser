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

use dearrow_browser_api::unsync::{ChannelFetchProgress, InnertubeChannel};
use cloneable_errors::{bail, ErrorContext, ResContext};
use gloo_console::warn;
use yew::prelude::*;

use crate::components::tables::{details::*, switch::*};
use crate::constants::REQWEST_CLIENT;
use crate::contexts::{StatusContext, WindowContext};
use crate::hooks::{
    use_async_loop, use_async_suspension, use_location_state, IterationResult, LoopControl,
};
use crate::utils::{RcEq, ReqwestResponseExt};

#[function_component]
fn ChannelDetails(props: &ChannelPageProps) -> HtmlResult {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let status: StatusContext = use_context().expect("StatusResponse should be defined");
    let result: Rc<Result<InnertubeChannel, ErrorContext>> = use_async_suspension(
        |(channel, _)| async move {
            let url = window_context.origin_join_segments(&["innertube", "channel", &channel]);
            loop {
                let resp = REQWEST_CLIENT
                    .get(url.clone())
                    .header("Accept", "application/json")
                    .send()
                    .await
                    .context("Failed to send the request")?;
                if resp.status().as_u16() == 333 {
                    // Not Ready Yet
                    continue;
                }
                return resp
                    .check_status()
                    .await?
                    .json()
                    .await
                    .context("Failed to deserialize response");
            }
        },
        (props.channel.clone(), status.map(|s| s.last_updated)),
    )?;

    Ok(match *result {
        Ok(ref channel) => html! {
            <>
                <div>{"Channel name: "}<a target="_blank" href={format!("https://youtube.com/channel/{}", channel.ucid)}>{channel.channel_name.clone()}</a></div>
                <div>{format!("Videos: {} plain, {} VODs, {} shorts, {} releases; {} total", channel.num_videos, channel.num_vods, channel.num_shorts, channel.num_releases, channel.total_videos)}</div>
            </>
        },
        Err(ref e) => html! {
            <>
                <div>{"Failed to fetch channel data"}<br/><pre>{format!("{e:?}")}</pre></div>
            </>
        },
    })
}

#[derive(Properties, PartialEq, Clone)]
pub struct ChannelPageProps {
    pub channel: AttrValue,
}

#[derive(Clone, Default)]
enum ChannelLoadingStatus {
    #[default]
    LoadingInitial,
    LoadingProgress(ChannelFetchProgress),
    Ready(DetailSlice),
    Failed(ErrorContext),
}

#[function_component]
pub fn ChannelPage(props: &ChannelPageProps) -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let state = use_location_state().get_state();

    let url_and_mode = use_memo(
        (state.detail_table_mode, props.channel.clone()),
        |(dtm, channel)| {
            DetailType::try_from(*dtm).ok().map(|dtm| match dtm {
                DetailType::Title => (
                    Rc::new(
                        window_context.origin_join_segments(&["api", "titles", "channel", channel]),
                    ),
                    dtm,
                ),
                DetailType::Thumbnail => (
                    Rc::new(window_context.origin_join_segments(&[
                        "api",
                        "thumbnails",
                        "channel",
                        channel,
                    ])),
                    dtm,
                ),
            })
        },
    );

    let detail_status = use_async_loop(
        |url_and_mode, ()| async move {
            async move {
                let Some((url, mode)) = url_and_mode.as_ref() else {
                    bail!("Invalid mode")
                };
                let resp = REQWEST_CLIENT
                    .get((**url).clone())
                    .header("Accept", "application/json")
                    .send()
                    .await
                    .context("Failed to send the request")?;
                if resp.status().as_u16() == 333 {
                    // Not Ready Yet
                    let status = match resp.json().await {
                        Ok(progress) => ChannelLoadingStatus::LoadingProgress(progress),
                        Err(err) => {
                            warn!(format!(
                                "Failed to deserialize '333 Not Ready Yet' json response: {err}"
                            ));
                            ChannelLoadingStatus::LoadingInitial
                        }
                    };
                    Ok(IterationResult {
                        result: status,
                        control: LoopControl::Continue,
                        state: (),
                    })
                } else {
                    let resp = resp.check_status().await?;
                    let mut slice = match mode {
                        DetailType::Thumbnail => DetailSlice::Thumbnails(RcEq(
                            resp.json()
                                .await
                                .context("Failed to deserialize response")?,
                        )),
                        DetailType::Title => DetailSlice::Titles(RcEq(
                            resp.json()
                                .await
                                .context("Failed to deserialize response")?,
                        )),
                    };
                    match slice {
                        DetailSlice::Thumbnails(ref mut list) => Rc::get_mut(&mut list.0)
                            .expect("should be get mutable reference here")
                            .sort_unstable_by(|a, b| b.time_submitted.cmp(&a.time_submitted)),
                        DetailSlice::Titles(ref mut list) => Rc::get_mut(&mut list.0)
                            .expect("should be get mutable reference here")
                            .sort_unstable_by(|a, b| b.time_submitted.cmp(&a.time_submitted)),
                    }
                    Ok(IterationResult {
                        result: ChannelLoadingStatus::Ready(slice),
                        control: LoopControl::Terminate,
                        state: (),
                    })
                }
            }
            .await
            .unwrap_or_else(|err| IterationResult {
                result: ChannelLoadingStatus::Failed(err),
                control: LoopControl::Terminate,
                state: (),
            })
        },
        url_and_mode,
    );

    let details_fallback = html! {
        <div><b>{"Loading..."}</b></div>
    };

    let table_html = match *detail_status {
        ChannelLoadingStatus::LoadingInitial | ChannelLoadingStatus::LoadingProgress(..) => html! {
                <center>
                    <b>{"Loading..."}</b><br />
                    if let ChannelLoadingStatus::LoadingProgress(ref loading_status) = *detail_status {
                        <em>{format!("videos: {} new videos fetched, {} pulled from fscache", loading_status.videos.videos_fetched, loading_status.videos.videos_in_fscache)}</em><br />
                        <em>{format!("VODs: {} new videos fetched, {} pulled from fscache", loading_status.vods.videos_fetched, loading_status.vods.videos_in_fscache)}</em><br />
                        <em>{format!("shorts: {} new videos fetched, {} pulled from fscache", loading_status.shorts.videos_fetched, loading_status.shorts.videos_in_fscache)}</em><br />
                        <em>{format!("releases (tab): {} new videos fetched, {} pulled from fscache", loading_status.releases_tab.videos_fetched, loading_status.releases_tab.videos_in_fscache)}</em><br />
                        <em>{format!("releases (home page): {} new videos fetched, {} pulled from fscache", loading_status.releases_home.videos_fetched, loading_status.releases_home.videos_in_fscache)}</em><br /><br />
                    }
                    {"Loading this page for the first time for a given channel will take a while, especially for channels with lots of videos."}<br />
                    {"Subsequent requests for this channel should be quick for everyone, until the cache is manually cleared."}
                </center>
        },
        ChannelLoadingStatus::Failed(ref err) => html! {
            <center>
                <b>{"Failed to fetch details from the API :/"}</b>
                <pre>{format!("{err:?}")}</pre>
            </center>
        },
        ChannelLoadingStatus::Ready(ref details) => html! {
            <BasePaginatedDetailTableRenderer details={details.clone()} />
        },
    };

    let entry_count = if let ChannelLoadingStatus::Ready(ref details) = *detail_status {
        Some(details.len())
    } else {
        None
    };

    html! {
        <>
            <div class="page-details">
                <div class="info-table">
                    <Suspense fallback={details_fallback}><ChannelDetails ..{props.clone()} /></Suspense>
                </div>
            </div>
            <TableModeSwitch {entry_count} types={ModeSubtype::Details} />
            {table_html}
        </>
    }
}
