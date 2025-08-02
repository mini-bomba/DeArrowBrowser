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
use dearrow_browser_api::unsync::ApiCasualTitle;
use yew::{function_component, html, Html};

use crate::{
    components::{icon::{Icon, IconType}, tables::r#trait::{RowProps, TableRender}, youtube::{OriginalTitle, YoutubeVideoLink}},
    settings::Settings,
    utils::render_datetime,
};

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct CasualTableSettings {
    pub hide_videoid: bool,
}

impl TableRender for ApiCasualTitle {
    type Settings = CasualTableSettings;
    type RowRenderer = CasualRow;
    const CLASS: &str = "causal-titles";

    fn render_header(settings: Self::Settings, _: &Settings) -> Html {
        html! {<>
            if !settings.hide_videoid {
                <th>{"Video ID"}</th>
            }
            <th class="title-col casual">{"Title"}</th>
            <th>{"Votes"}</th>
            <th>{"First submitted"}</th>
        </>}
    }
}

#[function_component]
pub fn CasualRow(props: &RowProps<ApiCasualTitle>) -> Html {
    let title = props.item();
    let settings = &props.settings;
    let timestamp = DateTime::from_timestamp_millis(title.first_submitted)
        .map_or_else(|| title.first_submitted.to_string(), render_datetime);

    html! {<>
        if !settings.hide_videoid {
            <td class="monospaced"><YoutubeVideoLink videoid={title.video_id.clone()} multiline={false} /></td>
        }
        <td class="title-col casual">
            if let Some(title) = &title.title {
                {title}
            } else {
                <Icon
                    r#type={IconType::NullTitle}
                    tooltip={"There's no title saved in the database, DeArrow Browser is showing the current original title instead."}
                />
                <span class="null-title"><OriginalTitle videoid={title.video_id.clone()} /></span>
            }
        </td>
        <td class="casual-votes-col monospaced">
            {for title.votes.iter().map(|(c, v)| html! {<>
                <span>{c}</span>
                <span>{v.to_string()}</span>
            </>})}
        </td>
        <td>{timestamp}</td>
    </>}
}
