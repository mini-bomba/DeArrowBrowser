/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024-2025 mini_bomba
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
use gloo_console::warn;
use wasm_bindgen::prelude::Closure;
use yew::platform::spawn_local;
use yew::prelude::*;

use crate::constants::COMMIT_LINK;
use crate::thumbnails::common::ThumbgenStats;
use crate::utils_common::Interval;
use crate::worker_api::WorkerStats;
use crate::worker_client::WorkerState;
use crate::{built_info, constants, worker_client};
use crate::contexts::{StatusContext, WindowContext};
use crate::thumbnails::components::{
    Thumbgen, ThumbgenContext, ThumbgenRefreshContext,
};
use crate::utils_app::{render_datetime, RenderNumber, SimpleLoadState};

macro_rules! number_hoverswitch {
    ($switch_element: tt, $n: expr) => {
        if $n >= 1000 {
            html! {
                <$switch_element class="hoverswitch">
                    <span>{$n.abbreviate_int()}</span>
                    <span>{$n.render_int()}</span>
                </$switch_element>
            }
        } else {
            html! {
                <$switch_element>{$n}</$switch_element>
            }
        }
    };
}

enum WorkerStatus {
    Initializing,
    Ready {
        stats: SimpleLoadState<WorkerStats>,
    },
    Failed {
        reason: AttrValue,
    }
}

pub struct ClientStatus {
    worker: WorkerState,
    thumbgen: ThumbgenContext,

    worker_status: WorkerStatus,
    thumbgen_stats: SimpleLoadState<ThumbgenStats>,
    version: u8,
    clear_thumb_errors: Callback<MouseEvent>,

    _worker_handle: ContextHandle<WorkerState>,
    _thumbgen_handle: ContextHandle<ThumbgenContext>,
    _refresh_interval: Interval<dyn Fn()>,
}

impl ClientStatus {
    fn refresh(&mut self, ctx: &Context<Self>) -> bool {
        let mut should_refresh = false;
        let scope = ctx.link();
        let version = self.version.wrapping_add(1);
        self.version = version;

        match (&self.worker, &mut self.worker_status) {
            (WorkerState::Loading, WorkerStatus::Initializing) | (WorkerState::Failed(..), WorkerStatus::Failed { .. }) => (),
            (WorkerState::Loading, status) => {
                *status = WorkerStatus::Initializing;
                should_refresh = true;
            },
            (WorkerState::Ready(client), WorkerStatus::Ready { .. }) => {
                let client = client.clone();
                let scope = scope.clone();
                spawn_local(async move {
                    scope.send_message(ClientStatusMessage::WorkerStatsUpdated { 
                        stats: client.get_stats().await,
                        version,
                    });
                });
            },
            (WorkerState::Ready(client), status) => {
                *status = WorkerStatus::Ready { stats: SimpleLoadState::Loading };
                should_refresh = true;
                let client = client.clone();
                let scope = scope.clone();
                spawn_local(async move {
                    scope.send_message(ClientStatusMessage::WorkerStatsUpdated { 
                        stats: client.get_stats().await,
                        version,
                    });
                });
            },
            (WorkerState::Failed(err), status) => {
                *status = WorkerStatus::Failed {
                    reason: AttrValue::from(format!("{err:?}"))
                };
                should_refresh = true;
            }
        }

        match &self.thumbgen {
            None => (),
            Some(Thumbgen::Local { gen, .. }) => {
                self.thumbgen_stats = SimpleLoadState::Ready(gen.get_stats());
                should_refresh = true;
            },
            Some(Thumbgen::Remote(gen)) => {
                let gen = gen.clone();
                let scope = scope.clone();
                spawn_local(async move {
                    scope.send_message(ClientStatusMessage::ThumbgenStatsUpdated { 
                        stats: gen.get_stats().await,
                        version,
                    });
                });
            }
        }

        should_refresh
    }
}

pub enum ClientStatusMessage {
    WorkerUpdated(WorkerState),
    ThumbgenUpdated(ThumbgenContext),
    RefreshStats,
    ClearThumbgenErrors,

    WorkerStatsUpdated { stats: Result<WorkerStats, worker_client::Error>, version: u8 },
    ThumbgenStatsUpdated { stats: Result<ThumbgenStats, worker_client::Error>, version: u8 },
}

impl Component for ClientStatus{
    type Properties = ();
    type Message = ClientStatusMessage;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();

        let (worker, worker_handle) = scope.context(scope.callback(ClientStatusMessage::WorkerUpdated)).expect("Worker should be available");
        let (thumbgen, thumbgen_handle) = scope.context(scope.callback(ClientStatusMessage::ThumbgenUpdated)).expect("Thumbgen should be available");

        let mut this = Self {
            worker,
            thumbgen,

            worker_status: WorkerStatus::Initializing,
            thumbgen_stats: SimpleLoadState::Loading,
            version: 0,
            clear_thumb_errors: scope.callback(|_| ClientStatusMessage::ClearThumbgenErrors),

            _worker_handle: worker_handle,
            _thumbgen_handle: thumbgen_handle,
            _refresh_interval: {
                let scope = scope.clone();
                Interval::new(Closure::new(move || scope.send_message(ClientStatusMessage::RefreshStats)), 5*1000)
            },
        };
        this.refresh(ctx);
        this
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ClientStatusMessage::WorkerUpdated(new_worker) => {
                self.worker = new_worker;
                self.refresh(ctx)
            },
            ClientStatusMessage::ThumbgenUpdated(new_thumbgen) => {
                self.thumbgen = new_thumbgen;
                self.refresh(ctx)
            },
            ClientStatusMessage::RefreshStats => {
                self.refresh(ctx)
            },
            ClientStatusMessage::ClearThumbgenErrors => {
                if let Some(thumbgen) = &self.thumbgen {
                    thumbgen.clear_errors();
                    ctx.link()
                        .context::<ThumbgenRefreshContext>(Callback::noop())
                        .unwrap()
                        .0
                        .trigger_refresh();
                    self.refresh(ctx)
                } else {
                    false
                }
            }
            ClientStatusMessage::WorkerStatsUpdated { stats: new_stats, version } => {
                if self.version != version {
                    return false;
                }
                if let WorkerStatus::Ready { ref mut stats } = self.worker_status {
                    *stats = new_stats
                        .inspect_err(|e| warn!(format!("Failed to fetch worker stats: {e:?}")))
                        .ok()
                        .into();
                    true
                } else {
                    false
                }
            },
            ClientStatusMessage::ThumbgenStatsUpdated { stats, version } => {
                if self.version != version {
                    return false;
                }
                self.thumbgen_stats = stats
                    .inspect_err(|e| warn!(format!("Failed to fetch thumbgen stats: {e:?}")))
                    .ok()
                    .into();
                true
            }
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        let sw_status = match self.worker_status {
            WorkerStatus::Initializing => "Initializing",
            WorkerStatus::Ready { .. } => "Ready",
            WorkerStatus::Failed { .. } => "Disabled",
        };
        html! {<>
            <h4>{"Build info"}</h4>
            <table>
                <tr>
                    <th>{"Version"}</th>
                    <td>{built_info::PKG_VERSION}</td>
                </tr>
                <tr>
                    <th>{"Git hash"}</th>
                    <td>
                        if let Some(hash) = built_info::GIT_COMMIT_HASH {
                            <a href={*COMMIT_LINK} target="_blank">{&hash[..8]}</a>
                            if built_info::GIT_DIRTY == Some(true) {
                                {" "}<b>{"+ uncommitted changes"}</b>
                            }
                        } else {
                            <em>{"Unknown"}</em>
                        }
                    </td>
                </tr>
                <tr>
                    <th>{"Build date"}</th>
                    <td>
                        if let Some(dt) = *constants::BUILD_TIME {
                            {render_datetime(dt.into())}
                        } else {
                            <em>{"Unknown"}</em>
                        }
                    </td>
                </tr>
            </table>
            <h4>{"Shared worker"}</h4>
            <table>
                <tr>
                    <th>{"Status"}</th>
                    <td>{sw_status}</td>
                </tr>
                {match &self.worker_status {
                    WorkerStatus::Initializing => html! {},
                    WorkerStatus::Failed { reason } => html! {
                        <tr>
                            <th>{"Reason"}</th>
                            <td>{reason}</td>
                        </tr>
                    },
                    WorkerStatus::Ready { stats: SimpleLoadState::Loading } => html! {
                        <em>{"Loading worker stats..."}</em>
                    },
                    WorkerStatus::Ready { stats: SimpleLoadState::Failed(..) } => html! {
                        <em>{"Failed to load worker stats (check console)"}</em>
                    },
                    WorkerStatus::Ready { stats: SimpleLoadState::Ready(stats) } => html! {<>
                        <tr>
                            <th>{"Clients connected"}</th>
                            <td>{stats.clients}</td>
                        </tr>
                        <tr>
                            <th>{"This client's thumbrefs"}</th>
                            <td>{stats.this_client_refs}</td>
                        </tr>
                    </>},
                }}
            </table>
            <h4>{"Thumbnail generator"}</h4>
            {match &self.thumbgen_stats {
                SimpleLoadState::Loading => html! {
                    <em>{"Loading..."}</em>
                },
                SimpleLoadState::Failed(..) => html! {
                    <em>{"Failed to load thumbgen stats (check console)"}</em>
                },
                SimpleLoadState::Ready(stats) => html! {
                    <table>
                        <tr>
                            <th>{"Total entries"}</th>
                            <td>{stats.total}</td>
                        </tr>
                        <tr>
                            <th>{"Pending thumbnails"}</th>
                            <td>{stats.pending}</td>
                        </tr>
                        <tr>
                            <th>{"Cached thumbnails"}</th>
                            <td>{format!("{} ({} in use)", stats.thumbs, stats.in_use)}</td>
                        </tr>
                        <tr>
                            <th>{"Cached errors "}<button onclick={&self.clear_thumb_errors}>{"Clear"}</button></th>
                            <td>{stats.errors}</td>
                        </tr>
                    </table>
                },
            }}
        </>}
    }
}

#[function_component]
fn ServerStatus() -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let status: StatusContext = use_context().expect("StatusContext should be defined");

    let server_version: Rc<Option<AttrValue>> = use_memo(status.clone(), |status| {
        let status = status.as_ref()?;

        if let Some(git_hash) = &status.server_git_hash {
            let short_hash = &git_hash[..8];
            match (&status.server_version, &status.server_git_dirty) {
                (Some(version), Some(true)) => Some(AttrValue::from(format!("{version} / {short_hash} + changes"))),
                (Some(version), _)          => Some(AttrValue::from(format!("{version} / {short_hash}"))),
                (None,          Some(true)) => Some(AttrValue::from(format!("{git_hash} + changes"))),
                (None,          _)          => Some(AttrValue::Rc(git_hash.clone())),
            }
        } else {
            status.server_version.clone().map(AttrValue::Rc)
        }
    });

    let errors_url: Rc<AttrValue> = use_memo(window_context, |wc| {
        wc.origin_join_segments(&["api", "errors"])
            .as_str()
            .to_owned()
            .into()
    });

    let Some(status) = status else {
        return html! {
            <em>{"Loading..."}</em>
        }
    };

    let server_version = html! {
        if let Some(version) = &*server_version {
            {version}
        } else {
            <em>{"Unknown"}</em>
        }
    };

    html! {<>
        <h4>{"Build info"}</h4>
        <table>
            <tr>
                <th>{"Brand"}</th>
                <td>
                if let Some(brand) = &status.server_brand {
                    {brand}
                } else {
                    <em>{"Unknown"}</em>
                }
                </td>
            </tr>
            <tr>
                <th>{"Version"}</th>
                <td>
                if let Some(url) = &status.server_url {
                    <a href={url.clone()}>{server_version}</a>
                } else {
                    {server_version}
                }
                </td>
            </tr>
            <tr>
                <th>{"Build date"}</th>
                <td>
                    if let Some(dt) = status.server_build_timestamp.and_then(|t| DateTime::from_timestamp(t, 0)) {
                        {render_datetime(dt)}
                    } else {
                        <em>{"Unknown"}</em>
                    }
                </td>
            </tr>
        </table>
        <h4>{"Server status"}</h4>
        <table>
            if let Some(ts) = status.server_startup_timestamp {
                <tr>
                    <th>{"Server started at"}</th>
                    <td>
                        if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                            {render_datetime(dt)}
                        } else {
                            <em>{"Failed to parse"}</em>
                        }
                    </td>
                </tr>
            }
            if let Some(ts) = status.last_updated {
                <tr>
                    <th>{"Last update"}</th>
                    <td>
                        if let Some(dt) = DateTime::from_timestamp_millis(ts) {
                            {render_datetime(dt)}
                            if status.updating_now {
                                <b>{", update in progress"}</b>
                            }
                        } else {
                            <em>{"Failed to parse"}</em>
                        }
                    </td>
                </tr>
            }
            if let Some(ts) = status.last_modified {
                <tr>
                    <th>{"DB snapshot taken at"}</th>
                    <td>
                        if let Some(dt) = DateTime::from_timestamp_millis(ts) {
                            {render_datetime(dt)}
                        } else {
                            <em>{"Failed to parse"}</em>
                        }
                    </td>
                </tr>
            }
            if let Some(count) = status.titles {
                <tr class="hoverswitch-trigger">
                    <th>{"Title count"}</th>
                    {number_hoverswitch!(td, count)}
                </tr>
            }
            if let Some(count) = status.thumbnails {
                <tr class="hoverswitch-trigger">
                    <th>{"Thumbnail count"}</th>
                    {number_hoverswitch!(td, count)}
                </tr>
            }
            if let Some(count) = status.casual_titles {
                <tr class="hoverswitch-trigger">
                    <th>{"Casual title count"}</th>
                    {number_hoverswitch!(td, count)}
                </tr>
            }
            if let Some(count) = status.usernames {
                <tr class="hoverswitch-trigger">
                    <th>{"Username count"}</th>
                    {number_hoverswitch!(td, count)}
                </tr>
            }
            if let Some(count) = status.vip_users {
                <tr class="hoverswitch-trigger">
                    <th>{"VIPs"}</th>
                    {number_hoverswitch!(td, count)}
                </tr>
            }
            if let Some(count) = status.warnings {
                <tr class="hoverswitch-trigger">
                    <th>{"Warnings"}</th>
                    {number_hoverswitch!(td, count)}
                </tr>
            }
            if let Some(count) = status.string_count {
                <tr class="hoverswitch-trigger">
                    <th>{"Unique strings"}</th>
                    {number_hoverswitch!(td, count)}
                </tr>
            }
            if let Some(count) = status.video_infos {
                <tr class="hoverswitch-trigger">
                    <th>{"Videos with durations"}</th>
                    {number_hoverswitch!(td, count)}
                </tr>
            }
            if let Some(count) = status.uncut_segments {
                <tr class="hoverswitch-trigger">
                    <th>{"Unmarked video segments"}</th>
                    {number_hoverswitch!(td, count)}
                </tr>
            }
            if let (Some(mem), Some(fs)) = (status.cached_channels, status.fscached_channels) {
                <tr class="hoverswitch-trigger">
                    <th>{"Cached channels"}</th>
                    if mem >= 1000 || fs >= 1000 {
                        <td class="hoverswitch">
                            <span>{mem.abbreviate_int()}{" / "}{fs.abbreviate_int()}</span>
                            <span>{mem.render_int()}{" / "}{fs.render_int()}</span>
                        </td>
                    } else {
                        <td>{mem}{" / "}{fs}</td>
                    }
                </tr>
            }
            if let Some(count) = status.errors {
                <tr class="hoverswitch-trigger">
                    <th>{"Parse errors"}</th>
                    <td>
                        {number_hoverswitch!(span, count)}{" "}
                        <a href={(*errors_url).clone()} target="_blank">{"(view)"}</a>
                    </td>
                </tr>
            }
        </table>
    </>}
}

#[function_component]
pub fn StatusModal() -> Html {
    html! {
        <div id="status-modal">
            <h2>{"About DeArrow Browser"}</h2>
            <div id="status-modal-client">
                <h3>{"Client information"}</h3>
                <ClientStatus />
            </div>
            <div id="status-modal-server">
                <h3>{"Server information"}</h3>
                <ServerStatus />
            </div>
        </div>
    }
}
