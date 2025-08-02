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
use yew::platform::spawn_local;
use yew::prelude::*;
use yew_hooks::{use_async, use_interval};

use crate::{built_info, constants};
use crate::contexts::{StatusContext, WindowContext};
use crate::thumbnails::components::{
    TRExt, Thumbgen, ThumbgenContext, ThumbgenContextExt, ThumbgenRefreshContext,
};
use crate::utils::{render_datetime, RenderNumber};

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

#[function_component]
pub fn StatusModal() -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let status: StatusContext = use_context().expect("StatusContext should be defined");
    let thumbgen: ThumbgenContext = use_context().expect("ThumbgenContext should be available");
    let thumbgen_refresh: ThumbgenRefreshContext =
        use_context().expect("ThumbgenRefreshContext should be available");
    let update_clock: UseStateHandle<bool> = use_state(|| false);

    let errors_url: Rc<AttrValue> = use_memo(window_context, |wc| {
        wc.origin_join_segments(&["api", "errors"])
            .as_str()
            .to_owned()
            .into()
    });

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

    let server_version = html! {
        if let Some(version) = &*server_version {
            {version}
        } else {
            <em>{"Unknown"}</em>
        }
    };

    let thumbgen_impl = match &thumbgen {
        None => None,
        Some(Thumbgen::Remote(..)) => Some("SharedWorker"),
        Some(Thumbgen::Local { .. }) => Some("Local"),
    };

    let thumbgen_fallback_reason: Rc<Option<AttrValue>> = use_memo(
        match &thumbgen {
            Some(Thumbgen::Local { error, .. }) => Some(error.clone()),
            _ => None,
        },
        |err| err.as_ref().map(|err| format!("{:?}", err.0).into()),
    );

    let thumbgen_stats = {
        let thumbgen = thumbgen.clone();
        use_async(async move {
            let Some(thumbgen) = thumbgen else {
                return Err(());
            };
            thumbgen.get_stats().await.map_err(|_| ())
        })
    };

    {
        let thumbgen_stats = thumbgen_stats.clone();
        use_memo((*update_clock, *thumbgen_refresh), |_| {
            thumbgen_stats.run();
        });
    }
    {
        let update_clock = update_clock.clone();
        use_interval(
            move || {
                update_clock.set(!*update_clock);
            },
            5 * 1000,
        );
    }

    let clear_errors = use_callback(thumbgen.clone(), move |_: MouseEvent, thumbgen| {
        let thumbgen = thumbgen.clone();
        let thumbgen_refresh = thumbgen_refresh.clone();
        spawn_local(async move {
            if let Some(ref thumbgen) = thumbgen {
                thumbgen.clear_errors().await;
                thumbgen_refresh.trigger_refresh();
            }
        });
    });

    html! {
        <div id="status-modal">
            <h2>{"About DeArrow Browser"}</h2>
            <div id="status-modal-client">
                <h3>{"Client information"}</h3>
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
                                <a href={format!("https://github.com/mini-bomba/DeArrowBrowser/commit/{hash}")} target="_blank">{&hash[..8]}</a>
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
                <h4>{"Thumbnail generator info"}</h4>
                <table>
                    <tr>
                        <th>{"Status"}</th>
                        <td>{thumbgen.get_status()}</td>
                    </tr>
                    if let Some(r#impl) = thumbgen_impl {
                    <tr>
                        <th>{"Implementation type"}</th>
                        <td>{r#impl}</td>
                    </tr>
                    }
                    if let Some(ref reason) = *thumbgen_fallback_reason {
                    <tr>
                        <th>{"Fallback reason"}</th>
                        <td>{reason}</td>
                    </tr>
                    }
                    if let Some(ref stats) = thumbgen_stats.data {
                    <tr>
                        <th>{"Cached entries"}</th>
                        <td>{stats.cache_stats.total}</td>
                    </tr>
                    <tr>
                        <th>{"Cached thumbnails"}</th>
                        <td>{format!("{} ({} in use)", stats.cache_stats.thumbs, stats.cache_stats.in_use)}</td>
                    </tr>
                    <tr>
                        <th>{"Cached errors "}<button onclick={clear_errors}>{"Clear"}</button></th>
                        <td>{stats.cache_stats.errors}</td>
                    </tr>
                    <tr>
                        <th>{"Pending thumbnails"}</th>
                        <td>{stats.cache_stats.pending}</td>
                    </tr>
                    if let Some(ref worker_stats) = stats.worker_stats {
                    <tr>
                        <th>{"Clients connected"}</th>
                        <td>{worker_stats.clients}</td>
                    </tr>
                    <tr>
                        <th>{"Refs owned by this client"}</th>
                        <td>{worker_stats.this_client_refs}</td>
                    </tr>
                    }
                    } else if thumbgen_stats.loading {
                    <em>{"Loading..."}</em>
                    }
                </table>
            </div>
            <div id="status-modal-server">
                <h3>{"Server information"}</h3>
                if let Some(status) = status {
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
                } else {
                    <em>{"Loading..."}</em>
                }
            </div>
        </div>
    }
}
