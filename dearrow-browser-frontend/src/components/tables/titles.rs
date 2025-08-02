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

use chrono::DateTime;
use dearrow_browser_api::unsync::ApiTitle;
use yew::{classes, function_component, html, use_callback, use_context, Html, MouseEvent};

use crate::{
    components::{
        icon::{Icon, IconType},
        modals::voting::{VotingDetail, VotingModal},
        tables::r#trait::{RowProps, TableRender},
        youtube::YoutubeVideoLink,
    },
    contexts::{ModalMessage, ModalRendererControls, SettingsContext, UserContext},
    score_col,
    settings::{Settings, TableLayout},
    userid_cell, username_cell,
    utils::render_datetime,
    uuid_cell,
};

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct TitleTableSettings {
    pub hide_videoid: bool,
    pub hide_username: bool,
    pub hide_userid: bool,
}

impl TableRender for ApiTitle {
    type Settings = TitleTableSettings;
    type RowRenderer = TitleRowRenderer;
    const CLASS: &str = "titles";

    fn render_header(settings: Self::Settings, _: &Settings) -> Html {
        html! {<>
            <th>{"Submitted"}</th>
            if !settings.hide_videoid {
                <th>{"Video ID"}</th>
            }
            <th class="title-col">{"Title"}</th>
            <th class="score-col">{"Score"}</th>
            <th>{"UUID"}</th>
            if !settings.hide_username {
                <th>{"Username"}</th>
            }
            if !settings.hide_userid {
                <th>{"User ID"}</th>
            }
        </>}
    }
}

fn title_flags(title: &ApiTitle) -> Html {
    html! {<>
        if title.votes_missing {
            <Icon r#type={IconType::VotesMissing} tooltip="Vote data is missing for this title - this title is hidden from the extension, attempting to vote on this title may cause server errors. DAB assumes the value of 0 or false for the missing fields." />
        }
        if title.removed || title.shadow_hidden {
            if title.removed {
                <Icon r#type={IconType::Removed} tooltip="This title was removed by a VIP" />
            }
            if title.shadow_hidden {
                <Icon r#type={IconType::ShadowHidden} tooltip="This title is shadowhidden" />
            }
        } else if title.votes - title.downvotes < -1 {
            <Icon r#type={IconType::Downvote} tooltip="This title was removed by the downvotes" />
        } else if title.votes < 0 {
            <Icon r#type={IconType::Replaced} tooltip="This title was replaced by the submitter" />
        } else if !title.locked && title.score < 0 {
            <Icon r#type={IconType::PartiallyHidden} tooltip="This title should only appear in submission menus (score below 0)" />
        }
        if title.unverified {
            <Icon r#type={IconType::Unverified} tooltip="This title was submitted by an unverified user (-1 score)" />
        }
        if title.locked {
            <Icon r#type={IconType::Locked} tooltip="This title was locked by a VIP" />
        }
        if title.vip {
            <Icon r#type={IconType::VIP} tooltip="This title was submitted by a VIP" />
        }
        if title.casual_mode {
            <Icon r#type={IconType::Casual} tooltip="This title was submitted by a casual mode user" />
        }
    </>}
}

#[function_component]
pub fn TitleRowRenderer(props: &RowProps<ApiTitle>) -> Html {
    let t = props.item();
    let modal_controls: ModalRendererControls =
        use_context().expect("ModalRendererControls should be available");
    let settings_context: SettingsContext =
        use_context().expect("SettingsContext should be available");
    let settings = settings_context.settings();
    let user_context: UserContext = use_context().expect("UserContext should be available");
    let voting_modal_trigger = {
        let modal_controls = modal_controls.clone();
        use_callback(
            (props.items.clone(), props.index),
            move |_: MouseEvent, (items, index)| {
                let detail = VotingDetail::Title(items[*index].clone());
                modal_controls.emit(ModalMessage::Open(html! {
                    <VotingModal {detail} />
                }));
            },
        )
    };
    let voting_modal_trigger = user_context.is_some().then_some(voting_modal_trigger);
    let score_col_class = classes!(
        "score-col",
        "hoverswitch-trigger",
        user_context.is_some().then_some("clickable")
    );
    let expanded_layout = settings.title_table_layout == TableLayout::Expanded;
    let compressed_layout = settings.title_table_layout == TableLayout::Compressed;
    let rows = if compressed_layout { "1" } else { "2" };
    let title_column_classes = classes!("title-col", compressed_layout.then_some("compressed"));
    html! {
        <>
            <td>{DateTime::from_timestamp_millis(t.time_submitted).map_or_else(|| t.time_submitted.to_string(), render_datetime)}</td>
            if !props.settings.hide_videoid {
                <td class="monospaced"><YoutubeVideoLink videoid={t.video_id.clone()} multiline={expanded_layout} /></td>
            }
            <td class={title_column_classes}>
                {t.title.clone()}
                if t.original {
                    if expanded_layout { <br /> } else {{""}}
                    <Icon r#type={IconType::Original} tooltip="This is the original video title" />
                }
            </td>
            <td class={score_col_class} onclick={voting_modal_trigger}>
                {score_col!(title_flags, t, expanded_layout)}
            </td>
            <td class="monospaced">{uuid_cell!(t.uuid, expanded_layout)}</td>
            if !props.settings.hide_username {
                <td>{username_cell!(t.username, rows)}</td>
            }
            if !props.settings.hide_userid {
                <td>{userid_cell!(t.user_id, rows, expanded_layout)}</td>
            }
        </>
    }
}
