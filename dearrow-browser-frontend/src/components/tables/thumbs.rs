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
use dearrow_browser_api::unsync::ApiThumbnail;
use yew::{classes, function_component, html, use_callback, use_context, use_memo, Callback, Html, MouseEvent};

use crate::{components::{icon::{Icon, IconType}, modals::{thumbnail::ThumbnailModal, voting::{VotingDetail, VotingModal}}, tables::r#trait::{RowProps, TableRender}, youtube::YoutubeVideoLink}, contexts::{ModalMessage, ModalRendererControls, SettingsContext, UserContext}, score_col, settings::{Settings, TableLayout}, thumbnails::components::{ContainerType, Thumbnail, ThumbnailCaption}, userid_cell, username_cell, utils_app::render_datetime, uuid_cell};


#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct ThumbTableSettings {
    pub hide_videoid: bool,
    pub hide_username: bool,
    pub hide_userid: bool,
}

impl TableRender for ApiThumbnail {
    type Settings = ThumbTableSettings;
    type RowRenderer = ThumbRowRenderer;
    const CLASS: &str = "thumbnails";

    fn render_header(render_settings: Self::Settings, user_settings: &Settings) -> Html {
        let rendering_thumbnails = user_settings.render_thumbnails_in_tables
            && user_settings.thumbnail_table_layout == TableLayout::Expanded;
        html! {<>
            <th>{"Submitted"}</th>
            if !render_settings.hide_videoid {
                <th>{"Video ID"}</th>
            }
            if rendering_thumbnails {
                <th class="thumbnail-header">{"Thumbnail"}</th>
            } else {
                <th>{"Timestamp"}</th>
            }
            <th class="score-col">{"Score"}</th>
            <th>{"UUID"}</th>
            if !render_settings.hide_username {
                <th>{"Username"}</th>
            }
            if !render_settings.hide_userid {
                <th>{"User ID"}</th>
            }
        </>}
    }
}

fn thumbnail_flags(thumb: &ApiThumbnail) -> Html {
    html! {
        <>
            if thumb.votes_missing {
                <Icon r#type={IconType::VotesMissing} tooltip="Vote data is missing for this thumbnail - this thumbnail is hidden from the extension, attempting to vote on this thumbnail may cause server errors. DAB assumes the value of 0 or false for the missing fields." />
            }
            if thumb.timestamp_missing {
                <Icon r#type={IconType::TimestampMissing} tooltip="This thumbnail is missing a timestamp despite being a custom thumbnail - this thumbnail will appear glitched in the voting menu of the extension." />
            }
            if thumb.removed || thumb.shadow_hidden {
                if thumb.removed {
                    <Icon r#type={IconType::Removed} tooltip="This thumbnail was removed by a VIP" />
                }
                if thumb.shadow_hidden {
                    <Icon r#type={IconType::ShadowHidden} tooltip="This thumbnail is shadowhidden" />
                }
            } else if thumb.votes - thumb.downvotes < -1 {
                <Icon r#type={IconType::Downvote} tooltip="This thumbnail was removed by the downvotes" />
            } else if !thumb.locked {
                if thumb.original && thumb.score < 1 {
                    <Icon r#type={IconType::Downvote} tooltip="This original thumbnail has insufficient score to be shown (requires >= 1 or lock)" />
                } else if thumb.score < 0 {
                    <Icon r#type={IconType::PartiallyHidden} tooltip="This thumbnail should only appear in submission menus (score below 0)" />
                }
            }
            if thumb.locked {
                <Icon r#type={IconType::Locked} tooltip="This thumbnail was locked by a VIP" />
            }
            if thumb.vip {
                <Icon r#type={IconType::VIP} tooltip="This thumbnail was submitted by a VIP" />
            }
            if thumb.casual_mode {
                <Icon r#type={IconType::Casual} tooltip="This thumbnail was submitted by a casual mode user" />
            }
        </>
    }
}

#[function_component]
pub fn ThumbRowRenderer(props: &RowProps<ApiThumbnail>) -> Html {
    let t = props.item();
    let modal_controls: ModalRendererControls =
        use_context().expect("ModalRendererControls should be available");
    let settings_context: SettingsContext =
        use_context().expect("SettingsContext should be available");
    let settings = settings_context.settings();
    let user_context: UserContext = use_context().expect("UserContext should be available");
    let timestamp_html = use_memo(t.timestamp, |timestamp| {
        if let Some(timestamp) = timestamp {
            html!{<span>{format!("{timestamp}")}</span>}
        } else {
            html! {<Icon r#type={IconType::Original} tooltip="This is the original video thumbnail" />}
        }
    });
    let voting_modal_trigger = {
        let modal_controls = modal_controls.clone();
        use_callback(
            (props.items.clone(), props.index),
            move |_: MouseEvent, (items, index)| {
                let detail = VotingDetail::Thumbnail(items[*index].clone());
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
    let expanded_layout = settings.thumbnail_table_layout == TableLayout::Expanded;
    let compressed_layout = settings.thumbnail_table_layout == TableLayout::Compressed;
    let rows = if compressed_layout { "1" } else { "2" };
    let render_thumbnails = settings.render_thumbnails_in_tables && expanded_layout;
    let onclick = {
        let timestamp = t.timestamp;
        let video_id = t.video_id.clone();
        Callback::from(move |_| {
            modal_controls.emit(ModalMessage::Open(html! {
                <ThumbnailModal video_id={video_id.clone()} {timestamp} />
            }));
        })
    };

    html! {<>
        <td>{DateTime::from_timestamp_millis(t.time_submitted).map_or_else(|| t.time_submitted.to_string(), render_datetime)}</td>
        if !props.settings.hide_videoid {
            <td class="monospaced"><YoutubeVideoLink videoid={t.video_id.clone()} multiline={expanded_layout} /></td>
        }
        if t.timestamp_missing {
            <td><Icon r#type={IconType::TimestampMissing} tooltip="This thumbnail entry is missing a timestamp and cannot be rendered" /></td>
        } else if render_thumbnails {
            <Thumbnail video_id={t.video_id.clone()} timestamp={t.timestamp} caption={ThumbnailCaption::Html((*timestamp_html).clone())} container_type={ContainerType::td} />
        } else {
            <td {onclick} class="clickable">{(*timestamp_html).clone()}</td>
        }
        <td class={score_col_class} onclick={voting_modal_trigger}>
            {score_col!(thumbnail_flags, t, expanded_layout)}
        </td>
        <td class="monospaced">{uuid_cell!(t.uuid, expanded_layout)}</td>
        if !props.settings.hide_username {
            <td>{username_cell!(t.username, rows, expanded_layout)}</td>
        }
        if !props.settings.hide_userid {
            <td>{userid_cell!(t.user_id, rows, expanded_layout)}</td>
        }
    </>}
}
