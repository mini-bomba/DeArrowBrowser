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
use chrono::DateTime;
use yew::prelude::*;
use yew_router::prelude::Link;

use crate::components::modals::{settings::SettingsModal, status::StatusModal, ModalMessage};
use crate::components::icon::*;
use crate::contexts::*;
use crate::pages::MainRoute;
use crate::utils::render_datetime_with_delta;

#[function_component]
pub fn Header() -> Html {
    let modal_controls: ModalRendererControls = use_context().expect("Header should be placed inside a ModalRenderer");
    let open_settings_modal = use_callback(modal_controls, |_, modal_controls| {
        modal_controls.emit(ModalMessage::Open(html! {<SettingsModal />}));
    });

    html! {
        <div id="header">
            <Link<MainRoute> to={MainRoute::Home}><img src="/icon/logo.svg" /></Link<MainRoute>>
            <div>
                <h1 class="undecorated-link"><Link<MainRoute> to={MainRoute::Home}>{"DeArrow Browser"}</Link<MainRoute>></h1>
                <span id="settings-button" class="clickable" onclick={open_settings_modal}><Icon r#type={IconType::Settings} tooltip={"Open settings"} /></span>
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
                    <td>{last_updated} if status.is_some_and(|s| s.updating_now) { <b>{", update in progress"}</b> }</td>
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
