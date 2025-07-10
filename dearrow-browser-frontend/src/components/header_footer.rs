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

use chrono::{DateTime, Datelike, Local};
use yew::prelude::*;
use yew::virtual_dom::VList;
use yew_router::hooks::use_navigator;
use yew_router::prelude::Link;

use crate::components::modals::{async_tasks::AsyncTasksModal, settings::SettingsModal, status::StatusModal, ModalMessage};
use crate::components::icon::*;
use crate::{constants, contexts::*};
use crate::pages::MainRoute;
use crate::utils::render_datetime_with_delta;

#[function_component]
pub fn Header() -> Html {
    let navigator = use_navigator().expect("Header should be placed in a Router");
    let modal_controls: ModalRendererControls = use_context().expect("Header should be placed inside a ModalRenderer");
    let settings_context: SettingsContext = use_context().expect("Header should be placed inside a SettingsProvider");
    let settings = settings_context.settings();
    let user_context: UserContext = use_context().expect("Header should be placed inside a SettingsProvider");
    let async_tasks_view: AsyncTaskList = use_context().expect("Header should be placed inside an AsyncTaskList");
    let open_settings_modal = use_callback(modal_controls.clone(), |_, modal_controls| {
        modal_controls.emit(ModalMessage::Open(html! {<SettingsModal />}));
    });
    let open_user_page = use_callback(user_context.as_ref().map(|d| d.user_id.clone()), move |_: MouseEvent, public_id| {
        if let Some(public_id) = public_id {
            navigator.push(&MainRoute::User { id: AttrValue::Rc(public_id.clone()) });
        }
    });
    let open_async_tasks_modal = use_callback(modal_controls, |_, modal_controls| {
        modal_controls.emit(ModalMessage::Open(html! {<AsyncTasksModal />}));
    });

    let task_badge: Rc<Html> = use_memo(async_tasks_view.clone(), |async_tasks_view| {
        let task_counts = async_tasks_view.count();
        let mut segments = Vec::with_capacity(3);
        if task_counts.pending != 0 {
            segments.push(html! {
                <span>
                    {task_counts.pending.to_string()}
                    <Icon r#type={IconType::Wait} />
                </span>
            });
        }
        if task_counts.success != 0 {
            segments.push(html! {
                <span>
                    {task_counts.success.to_string()}
                    <Icon r#type={IconType::Done} />
                </span>
            });
        }
        if task_counts.failed != 0 {
            segments.push(html! {
                <span>
                    {task_counts.failed.to_string()}
                    <Icon r#type={IconType::Removed} />
                </span>
            });
        }
        VList::with_children(segments, None).into()
    });

    let header_classes = classes!(settings.sticky_headers.then_some("sticky"));

    html! {
        <div id="header" class={header_classes}>
            <Link<MainRoute> to={MainRoute::Home}><img src="/icon/logo.svg" /></Link<MainRoute>>
            <div>
                <h1 class="undecorated-link"><Link<MainRoute> to={MainRoute::Home}>{"DeArrow Browser"}</Link<MainRoute>></h1>
                if !async_tasks_view.tasks.is_empty() {
                    <div id="async-tasks-badge" class="clickable header-badge" onclick={open_async_tasks_modal}>
                        {(*task_badge).clone()}
                    </div>
                }
                if let Some(user_data) = user_context {
                    <div id="current-user-badge" class="clickable header-badge" onclick={open_user_page}>
                        <span>
                            if let Some(Ok(user_details)) = user_data.data {
                                if let Some(username) = user_details.username.clone().filter(|name| *name != user_data.user_id) {
                                    <span id="current-user-name">{username}</span>
                                } else {
                                    <em>{"No username"}</em>
                                }
                                if user_details.vip {
                                    {" "}<Icon r#type={IconType::VIP} tooltip={Some("VIP user")} />
                                }
                            } else if let Some(Err(..)) = user_data.data {
                                <em>{"Error"}</em>
                            } else {
                                <em>{"Loading..."}</em>
                            } 
                        </span>
                        <span id="current-user-id">{user_data.user_id}</span>
                    </div>
                }
                <Icon r#type={IconType::Settings} tooltip={"Open settings"} onclick={open_settings_modal} id="settings-button" />
            </div>
        </div>
    }
}

#[function_component]
pub fn Footer() -> Html {
    let status: StatusContext = use_context().expect("StatusResponse should be defined");
    let _ = use_context::<UpdateClock>();
    let modal_controls: ModalRendererControls = use_context().expect("Footer should be placed inside a ModalRenderer");
    let open_version_modal = use_callback(modal_controls, |_, modal_controls| {
        modal_controls.emit(ModalMessage::Open(html! {<StatusModal />}));
    });

    let copyright = use_memo((), |()| {
        let dt = constants::COMMIT_TIME.or(*constants::BUILD_TIME);
        let year = match dt {
            Some(dt) => dt.year(),
            None => Local::now().year(),
        };
        AttrValue::from(format!(" © mini_bomba 2023-{year}, licensed under "))
    });

    let left_part = match status.as_ref() {
        None => html! {
            {"Loading..."}
        },
        Some(status) => {
            if status.last_modified.is_none() && status.last_updated.is_none() {
                let server_line = match (&status.server_brand, &status.server_version) {
                    (None, None) => html! {
                        <em>{"Unknown"}</em>
                    },
                    (None, Some(ver)) => html! {<>
                        <em>{"Unknown brand "}</em>{ver.clone()}
                    </>},
                    (Some(brand), None) => html! {
                        {brand.clone()}
                    },
                    (Some(brand), Some(ver)) => html! {<>
                        {brand.clone()}{" "}{ver.clone()}
                    </>},
                };
                // server doesn't provide snapshot timestamps
                // assume we have live data
                html! {
                    <table>
                        <tr><td>{"Backend server: "}{server_line}</td></tr>
                        <tr><td>{"Probably serving live data"}</td></tr>
                    </table>
                }
            } else {
                let last_updated = match status.last_updated.and_then(DateTime::from_timestamp_millis) {
                    None => AttrValue::from("Unknown"),
                    Some(time) => AttrValue::from(render_datetime_with_delta(time)),
                };
                let last_modified = match status.last_modified.and_then(DateTime::from_timestamp_millis) {
                    None => AttrValue::from("Unknown"),
                    Some(time) => AttrValue::from(render_datetime_with_delta(time)),
                };

                html! {
                    <table>
                        <tr>
                            <td>{"Last update:"}</td>
                            <td>{last_updated} if status.updating_now { <b>{", update in progress"}</b> }</td>
                        </tr>
                        <tr>
                            <td>{"Database snapshot taken at:"}</td>
                            <td>{last_modified}</td>
                        </tr>
                    </table>
                }
            }
        }
    };

    html! {
        <div id="footer">
            <div class="clickable" onclick={open_version_modal}>
                {left_part}
            </div>
            <span>
                <table>
                    <tr><td>
                        <a href="https://github.com/mini-bomba/DeArrowBrowser">{"DeArrow Browser"}</a>
                        {AttrValue::clone(&copyright)}
                        <a href="https://www.gnu.org/licenses/agpl-3.0.en.html">{"AGPL v3"}</a>
                    </td></tr>
                    <tr><td>
                        {"Uses DeArrow data licensed under "}
                        <a href="https://creativecommons.org/licenses/by-nc-sa/4.0/">{"CC BY-NC-SA 4.0"}</a>
                        {" from "}
                        <a href="https://dearrow.ajay.app/">{"https://dearrow.ajay.app/"}</a>
                        {"."}
                    </td></tr>
                </table>
            </span>
        </div>
    }
}
