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

use chrono::DateTime;
use yew::prelude::*;
use yew_router::hooks::use_navigator;

use crate::{contexts::*, pages::MainRoute, components::modals::{ModalMessage, status::StatusModal}, utils::render_datetime_with_delta};

#[function_component]
pub fn Header() -> Html {
    let navigator = use_navigator().expect("navigator should exist");
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");

    let go_home = {
        Callback::from(move |_| {
            navigator.push(&MainRoute::Home);
        })
    };

    html! {
        <div id="header">
            if let Some(url) = &window_context.logo_url {
                <img src={url} class="clickable" onclick={go_home.clone()} />
            }
            <div>
                <h1 class="clickable" onclick={go_home}>{"DeArrow Browser"}</h1>
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

    let last_updated = match status.as_ref().and_then(|status| DateTime::from_timestamp_millis(status.last_updated)) {
        None => AttrValue::from("..."),
        Some(time) => AttrValue::from(render_datetime_with_delta(time)),
    };
    let last_modified = match status.as_ref().and_then(|status| DateTime::from_timestamp_millis(status.last_modified)) {
        None => AttrValue::from("..."),
        Some(time) => AttrValue::from(render_datetime_with_delta(time)),
    };

    html! {
        <div id="footer">
            <table class="clickable" onclick={open_version_modal}>
                <tr>
                    <td>{"Last update:"}</td>
                    <td>{last_updated} if status.map(|s| s.updating_now).unwrap_or_default() { <b>{", update in progress"}</b> }</td>
                </tr>
                <tr>
                    <td>{"Database snapshot taken at:"}</td>
                    <td>{last_modified}</td>
                </tr>
            </table>
            <span>
                <table>
                    <tr><td>
                        <a href="https://github.com/mini-bomba/DeArrowBrowser">{"DeArrow Browser"}</a>
                        {" © mini_bomba 2023-2024, licensed under "}
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
