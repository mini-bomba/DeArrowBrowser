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

use dearrow_browser_api::unsync::{ApiCasualTitle, ApiThumbnail, ApiTitle, ChannelFetchProgress, InnertubeChannel};
use cloneable_errors::ResContext;
use gloo_console::error;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use strum::{IntoStaticStr, VariantArray};
use yew::platform::spawn_local;
use yew::prelude::*;

use crate::components::tables::remote::{Endpoint, RemotePaginatedTable};
use crate::components::tables::switch::*;
use crate::constants::REQWEST_CLIENT;
use crate::contexts::WindowContext;
use crate::hooks::
    use_location_state
;
use crate::utils_app::{CancelHandle, CancelWatcher, SimpleLoadState};
use crate::utils_common::{ReqwestResponseExt, ReqwestUrlExt};


struct ChannelDetails {
    origin: Url,

    data: SimpleLoadState<InnertubeChannel>,
    handle: CancelHandle,

    _wc_listener: ContextHandle<Rc<WindowContext>>,
}

impl ChannelDetails {
    fn refresh(&mut self, ctx: &Context<Self>) {
        self.data = SimpleLoadState::Loading;
        self.handle = CancelHandle::new();
        let channel = ctx.props().channel.clone();
        let url = self.origin.join_segments(&["innertube", "channel", &channel]).expect("Origin should be a valid base");
        let scope = ctx.link().clone();
        let watcher = self.handle.watch();
        spawn_local(async move {
            let res = async {
                loop {
                    if watcher.check() {
                        return Ok(None);
                    }
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
                    if watcher.check() {
                        return Ok(None);
                    }
                    return resp
                        .check_status()
                        .await?
                        .json::<InnertubeChannel>()
                        .await
                        .context("Failed to deserialize response")
                        .map(Some);
                }
            }.await;

            match res {
                Ok(None) => (),
                Err(err) => {
                    error!(format!("Failed to load channel data for channel {channel}: {err:?}"));
                    scope.send_message(ChannelDetailsMessage::DataFetched { data: SimpleLoadState::Failed(()), watcher });
                },
                Ok(Some(d)) => scope.send_message(ChannelDetailsMessage::DataFetched { data: SimpleLoadState::Ready(d), watcher }),
            }
        });
    }
}

enum ChannelDetailsMessage {
    OriginUpdated(Url),
    DataFetched{ data: SimpleLoadState<InnertubeChannel>, watcher: CancelWatcher },
}

impl Component for ChannelDetails {
    type Message = ChannelDetailsMessage;
    type Properties = ChannelPageProps;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();

        let (wc, wc_listener) = scope
            .context(scope.callback(|wc: Rc<WindowContext>| {
                ChannelDetailsMessage::OriginUpdated(wc.origin.clone())
            }))
            .expect("WindowContext should be available");

        let mut this = Self {
            origin: wc.origin.clone(),

            data: SimpleLoadState::Loading,
            handle: CancelHandle::new(),

            _wc_listener: wc_listener
        };
        this.refresh(ctx);
        this
    }

    fn view(&self, _: &Context<Self>) -> Html {
        match &self.data {
            SimpleLoadState::Loading => html! {<em>{"Loading..."}</em>},
            SimpleLoadState::Failed(()) => html! {<em>{"Failed to fetch channel data (see console)"}</em>},
            SimpleLoadState::Ready(channel) => html! {<>
                <div>
                    {"Channel name: "}
                    <a target="_blank" href={format!("https://youtube.com/channel/{}", channel.ucid)}>
                        {channel.channel_name.clone()}
                    </a>
                </div>
                <div>{format!(
                    "Videos: {} plain, {} VODs, {} shorts, {} releases; {} total",
                    channel.num_videos,
                    channel.num_vods,
                    channel.num_shorts,
                    channel.num_releases,
                    channel.total_videos,
                )}</div>
            </>}
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ChannelDetailsMessage::OriginUpdated(origin) => {
                if self.origin == origin {
                    false
                } else {
                    self.refresh(ctx);
                    true
                }
            },
            ChannelDetailsMessage::DataFetched { data, watcher } => {
                if self.handle.compare(&watcher) {
                    self.data = data;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if ctx.props().channel == old_props.channel {
            false
        } else {
            self.refresh(ctx);
            true
        }
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct ChannelPageProps {
    pub channel: AttrValue,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, VariantArray, IntoStaticStr, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
enum ChannelPageTab {
    #[default]
    Titles,
    Thumbnails,
    #[strum(serialize="Casual titles")]
    CasualTitles,
}

#[derive(PartialEq, Eq, Clone)]
struct ChannelTitles {
    channel: AttrValue,
}
#[derive(PartialEq, Eq, Clone)]
struct ChannelThumbs {
    channel: AttrValue,
}
#[derive(PartialEq, Eq, Clone)]
struct ChannelCasualTitles {
    channel: AttrValue,
}

fn render_channel_progress(progress: &ChannelFetchProgress) -> Html {
    html! {<>
        <b>{"Loading..."}</b><br />
        <em>{format!("videos: {} new videos fetched, {} pulled from fscache", progress.videos.videos_fetched, progress.videos.videos_in_fscache)}</em><br />
        <em>{format!("VODs: {} new videos fetched, {} pulled from fscache", progress.vods.videos_fetched, progress.vods.videos_in_fscache)}</em><br />
        <em>{format!("shorts: {} new videos fetched, {} pulled from fscache", progress.shorts.videos_fetched, progress.shorts.videos_in_fscache)}</em><br />
        <em>{format!("releases (tab): {} new videos fetched, {} pulled from fscache", progress.releases_tab.videos_fetched, progress.releases_tab.videos_in_fscache)}</em><br />
        <em>{format!("releases (home page): {} new videos fetched, {} pulled from fscache", progress.releases_home.videos_fetched, progress.releases_home.videos_in_fscache)}</em><br /><br />
        {"Loading this page for the first time for a given channel will take a while, especially for channels with lots of videos."}<br />
        {"Subsequent requests for this channel should be quick for everyone, until the cache is manually cleared."}
    </>}
}

impl Endpoint for ChannelTitles {
    type Item = ApiTitle;
    type LoadProgress = ChannelFetchProgress;

    fn create_url(&self, base_url: &Url) -> Url {
        base_url.join_segments(&["api", "titles", "channel", &self.channel]).expect("base should be a valid base")
    }
    fn render_load_progress(&self, progress: &Self::LoadProgress) -> Html {
        render_channel_progress(progress)
    }
}
impl Endpoint for ChannelThumbs {
    type Item = ApiThumbnail;
    type LoadProgress = ChannelFetchProgress;

    fn create_url(&self, base_url: &Url) -> Url {
        base_url.join_segments(&["api", "thumbnails", "channel", &self.channel]).expect("base should be a valid base")
    }
    fn render_load_progress(&self, progress: &Self::LoadProgress) -> Html {
        render_channel_progress(progress)
    }
}
impl Endpoint for ChannelCasualTitles {
    type Item = ApiCasualTitle;
    type LoadProgress = ChannelFetchProgress;

    fn create_url(&self, base_url: &Url) -> Url {
        base_url.join_segments(&["api", "casual_titles", "channel", &self.channel]).expect("base should be a valid base")
    }
    fn render_load_progress(&self, progress: &Self::LoadProgress) -> Html {
        render_channel_progress(progress)
    }
}

#[function_component]
pub fn ChannelPage(props: &ChannelPageProps) -> Html {
    let state = use_location_state().get_state::<ChannelPageTab>();
    let entry_count: UseStateHandle<Option<usize>> = use_state_eq(|| None);
    let callback = {
        let setter = entry_count.setter();
        use_callback((), move |new, ()| setter.set(new))
    };

    html! {
        <>
            <div class="page-details">
                <div class="info-table">
                    <ChannelDetails ..{props.clone()} />
                </div>
            </div>
            <TableModeSwitch<ChannelPageTab> entry_count={*entry_count} />
            {match state.tab {
                ChannelPageTab::Titles => html! {
                    <RemotePaginatedTable<ChannelTitles, ChannelPageTab>
                        endpoint={ChannelTitles { channel: props.channel.clone() }}
                        item_count_update={callback}
                    />
                },
                ChannelPageTab::Thumbnails => html! {
                    <RemotePaginatedTable<ChannelThumbs, ChannelPageTab>
                        endpoint={ChannelThumbs { channel: props.channel.clone() }}
                        item_count_update={callback}
                    />
                },
                ChannelPageTab::CasualTitles => html! {
                    <RemotePaginatedTable<ChannelCasualTitles, ChannelPageTab>
                        endpoint={ChannelCasualTitles { channel: props.channel.clone() }}
                        item_count_update={callback}
                    />
                },
            }}
        </>
    }
}
