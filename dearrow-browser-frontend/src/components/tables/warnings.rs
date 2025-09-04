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

use chrono::DateTime;
use dearrow_browser_api::unsync::{ApiWarning, Extension};
use yew::prelude::*;

use crate::{
    components::{
        links::userid_link,
        tables::r#trait::{RowProps, TableRender},
    },
    settings::Settings,
    utils_app::render_datetime,
};

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct WarningTableSettings {
    pub hide_issuer: bool,
    pub hide_receiver: bool,
}

impl TableRender for ApiWarning {
    type Settings = WarningTableSettings;
    type RowRenderer = WarningRow;
    const CLASS: &str = "warnings";

    fn render_header(settings: Self::Settings, _: &Settings) -> Html {
        html! {<>
            <th>{"Issued"}</th>
            <th>{"Message"}</th>
            if !settings.hide_issuer {
                <th>{"Issuer"}</th>
            }
            if !settings.hide_receiver {
                <th>{"Receiver"}</th>
            }
        </>}
    }
}

#[function_component]
pub fn WarningRow(props: &RowProps<ApiWarning>) -> Html {
    let warning = props.item();
    let settings = &props.settings;
    let timestamp = use_memo(warning.time_issued, |timestamp| {
        DateTime::from_timestamp_millis(*timestamp)
            .map_or_else(|| timestamp.to_string(), render_datetime)
    });
    let extension = match warning.extension {
        Extension::DeArrow => "for DeArrow",
        Extension::SponsorBlock => "for SponsorBlock",
    };
    let status = if warning.active {
        "Active"
    } else {
        "Acknowledged"
    };
    html! {<>
        <td>
            {timestamp}<br/>
            {extension}<br/>
            {status}
        </td>
        <td class="warning-message-col"><pre>{warning.message.clone()}</pre></td>
        if !settings.hide_issuer {
            <td>
                <textarea readonly=true cols=20 rows=3 ~value={warning.issuer_user_id.clone()} /><br/>
                if let Some(username) = warning.issuer_username.clone() {
                    <textarea readonly=true cols=20 rows=3 ~value={username} /><br/>
                }
                {userid_link(warning.issuer_user_id.clone().into())}
            </td>
        }
        if !settings.hide_receiver {
            <td>
                <textarea readonly=true cols=20 rows=3 ~value={warning.warned_user_id.clone()} /><br/>
                if let Some(username) = warning.warned_username.clone() {
                    <textarea readonly=true cols=20 rows=3 ~value={username} /><br/>
                }
                {userid_link(warning.warned_user_id.clone().into())}
            </td>
        }
    </>}
}
