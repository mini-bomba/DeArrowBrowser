/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2025 mini_bomba
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
use dearrow_browser_api::unsync::User;
use yew::{function_component, html, AttrValue, Html};
use yew_router::prelude::Link;

use crate::{
    components::{
        icon::{Icon, IconType},
        tables::r#trait::{RowProps, TableRender},
    }, pages::MainRoute, settings::Settings, utils_app::render_datetime
};

impl TableRender for User {
    type Settings = ();
    type RowRenderer = UserRow;
    const CLASS: &str = "user";

    fn render_header((): Self::Settings, _: &Settings) -> Html {
        html! {<>
            <th>{"User ID"}</th>
            <th>{"Flags"}</th>
            <th>{"Titles"}</th>
            <th>{"Thumbnails"}</th>
            <th>{"Last submission"}</th>
        </>}
    }
}

#[function_component]
pub fn UserRow(props: &RowProps<User>) -> Html {
    let user = props.item();
    let timestamp = user
        .last_submission
        .map_or("No submissions".to_string(), |ts| {
            DateTime::from_timestamp_millis(ts).map_or_else(|| ts.to_string(), render_datetime)
        });
    html! {<>
        <td>
            <Link<MainRoute> to={MainRoute::User { id: AttrValue::Rc(user.user_id.clone()) }}>
                <span class="monospaced">{&user.user_id}</span>
                <Icon r#type={IconType::DABLogo} />
            </Link<MainRoute>>
        </td>
        <td>
            if user.vip {
                <Icon r#type={IconType::VIP} tooltip="This user is a VIP" />
            }
            if user.username_locked {
                <Icon r#type={IconType::Locked} tooltip="This user cannot change their username" />
            }
            if user.active_warning_count > 0 {
                <Icon r#type={IconType::Warning} tooltip="This user has an active warning" />
            } else if user.warning_count > 0 {
                <Icon r#type={IconType::WarningInactive} tooltip="This user was previously warned" />
            }
        </td>
        <td>{user.title_count}</td>
        <td>{user.thumbnail_count}</td>
        <td>{timestamp}</td>
    </>}
}
