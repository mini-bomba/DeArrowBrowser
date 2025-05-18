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
use dearrow_browser_api::unsync::*;
use cloneable_errors::ErrorContext;
use reqwest::Url;
use yew::{prelude::*, suspense::SuspensionResult};

use crate::components::{
    icon::*,
    links::*,
    modals::{
        thumbnail::ThumbnailModal,
        voting::{VotingDetail, VotingModal},
    },
    tables::switch::PageSelect,
    youtube::YoutubeVideoLink,
};
use crate::contexts::{
    ModalMessage, ModalRendererControls, SettingsContext, StatusContext, UserContext,
};
use crate::hooks::{use_async_suspension, use_location_state};
use crate::settings::TableLayout;
use crate::thumbnails::components::{ContainerType, Thumbnail, ThumbnailCaption};
use crate::utils::{api_request, html_length, render_datetime, RcEq};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum DetailType {
    Title,
    Thumbnail,
}

pub enum DetailList {
    Thumbnails(Vec<ApiThumbnail>),
    Titles(Vec<ApiTitle>),
}

impl DetailList {
    pub fn len(&self) -> usize {
        match self {
            DetailList::Thumbnails(ref l) => l.len(),
            DetailList::Titles(ref l) => l.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            DetailList::Thumbnails(ref l) => l.is_empty(),
            DetailList::Titles(ref l) => l.is_empty(),
        }
    }
}

#[hook]
pub fn use_detail_download(
    url: Rc<Url>,
    mode: DetailType,
    sort: bool,
) -> SuspensionResult<Rc<Result<DetailSlice, ErrorContext>>> {
    let status: StatusContext = use_context().expect("StatusResponse should be defined");
    use_async_suspension(
        |(mode, url, sort, _)| async move {
            let mut result = match mode {
                DetailType::Thumbnail => {
                    DetailSlice::Thumbnails(RcEq(api_request((*url).clone()).await?))
                }
                DetailType::Title => DetailSlice::Titles(RcEq(api_request((*url).clone()).await?)),
            };
            if sort {
                // Sort by time submited, most to least recent
                match result {
                    DetailSlice::Thumbnails(ref mut list) => Rc::get_mut(&mut list.0)
                        .expect("should be get mutable reference here")
                        .sort_unstable_by(|a, b| b.time_submitted.cmp(&a.time_submitted)),
                    DetailSlice::Titles(ref mut list) => Rc::get_mut(&mut list.0)
                        .expect("should be get mutable reference here")
                        .sort_unstable_by(|a, b| b.time_submitted.cmp(&a.time_submitted)),
                }
            }
            Ok(result)
        },
        (mode, url, sort, status.map(|s| s.last_updated)),
    )
}

#[derive(Properties, PartialEq)]
pub struct BaseDetailTableRendererProps {
    pub details: DetailSlice,
    #[prop_or_default]
    pub hide_userid: bool,
    #[prop_or_default]
    pub hide_username: bool,
    #[prop_or_default]
    pub hide_videoid: bool,
}

#[derive(Properties, PartialEq)]
pub struct DetailTableRendererProps {
    pub url: Rc<Url>,
    pub mode: DetailType,
    #[prop_or_default]
    pub entry_count: Option<UseStateSetter<Option<usize>>>,
    #[prop_or_default]
    pub hide_userid: bool,
    #[prop_or_default]
    pub hide_username: bool,
    #[prop_or_default]
    pub hide_videoid: bool,
    #[prop_or(true)]
    pub sort: bool,
}

#[derive(Clone, PartialEq)]
pub enum DetailSlice {
    Thumbnails(RcEq<[ApiThumbnail]>),
    Titles(RcEq<[ApiTitle]>),
}

impl DetailSlice {
    pub fn len(&self) -> usize {
        match self {
            DetailSlice::Thumbnails(ref l) => l.len(),
            DetailSlice::Titles(ref l) => l.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            DetailSlice::Thumbnails(ref l) => l.is_empty(),
            DetailSlice::Titles(ref l) => l.is_empty(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DetailIndex {
    Page { size: usize, index: usize },
    All,
}

#[hook]
pub fn use_detail_slice(details: Option<DetailSlice>, index: DetailIndex) -> DetailSlice {
    (*use_memo((details, index), |(details, index)| {
        match (details, index) {
            (None, _) => DetailSlice::Thumbnails((&[] as &[ApiThumbnail]).into()), // dummy slice on error

            (Some(DetailSlice::Thumbnails(ref thumbs)), DetailIndex::Page { size, index }) => {
                DetailSlice::Thumbnails(
                    if size * (index + 1) > thumbs.len() {
                        thumbs.get(size * index..)
                    } else {
                        thumbs.get(size * index..size * (index + 1))
                    }
                    .unwrap_or(&[])
                    .into(),
                )
            }
            (Some(DetailSlice::Thumbnails(ref thumbs)), DetailIndex::All) => {
                DetailSlice::Thumbnails((&**thumbs).into())
            }

            (Some(DetailSlice::Titles(ref titles)), DetailIndex::Page { size, index }) => {
                DetailSlice::Titles(
                    if size * (index + 1) > titles.len() {
                        titles.get(size * index..)
                    } else {
                        titles.get(size * index..size * (index + 1))
                    }
                    .unwrap_or(&[])
                    .into(),
                )
            }
            (Some(DetailSlice::Titles(ref titles)), DetailIndex::All) => {
                DetailSlice::Titles((&**titles).into())
            }
        }
    }))
    .clone()
}

fn title_flags(title: &ApiTitle) -> Html {
    html! {
        <>
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
        </>
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
        </>
    }
}

/// This is for the compact mode, to avoid a trailing bar
///
/// Yes, this is stupid
/// Let me do stupid things
/// I'm having *fun*
fn bar_prepender_if_not_empty(html: Html) -> Html {
    if html_length(&html) == 0 {
        html! {}
    } else {
        html! {
            <>
                {" | "}
                {html}
            </>
        }
    }
}

macro_rules! detail_flags {
    (thumb, $detail:expr) => {
        thumbnail_flags($detail)
    };
    (title, $detail:expr) => {
        title_flags($detail)
    };
}

macro_rules! score_col {
    ($type:tt, $detail:expr, $expanded:expr) => {
        if $detail.votes_missing {
            html! {
                <>
                    <em>{"No data"}</em>
                    if $expanded {<br />} else {{" | "}}
                    {detail_flags!($type, $detail)}
                </>
            }
        } else if $expanded {
            html! {
                <>
                    <span class="hoverswitch">
                        <span>{$detail.score}</span>
                        <span><Icon r#type={IconType::Upvote} />{format!(" {} | {} ", $detail.votes, $detail.downvotes)}<Icon r#type={IconType::Downvote} /></span>
                    </span>
                    <br />
                    {detail_flags!($type, $detail)}
                </>
            }

        } else {
            html! {
                <>
                    {format!("{} | ", $detail.score)}<Icon r#type={IconType::Upvote} />{format!(" {} | {} ", $detail.votes, $detail.downvotes)}<Icon r#type={IconType::Downvote} />
                    {bar_prepender_if_not_empty(detail_flags!($type, $detail))}
                </>
            }
        }
    };
}

macro_rules! uuid_cell {
    ($uuid:expr, $multiline:expr) => {
        html! {
            <>
                {$uuid.clone()}
                if $multiline { <br /> } else {{" "}}
                {uuid_link($uuid.clone().into())}
            </>
        }
    };
}

macro_rules! userid_cell {
    ($userid:expr, $rows:expr, $multiline:expr) => {
        html! {
            <>
                <textarea readonly=true cols=16 rows={$rows} ~value={$userid.clone()} />
                if $multiline { <br /> } else {{" "}}
                {userid_link($userid.clone().into())}
            </>
        }
    };
}

macro_rules! username_cell {
    ($username:expr, $rows:expr) => {
        if let Some(ref name) = $username {
            html! {<textarea readonly=true cols=16 rows={$rows} ~value={name.to_string()} />}
        } else {
            html! {{"-"}}
        }
    };
}

#[derive(Properties, PartialEq, Clone)]
struct DetailTableRowProps {
    details: DetailSlice,
    index: usize,
    #[prop_or_default]
    pub hide_userid: bool,
    #[prop_or_default]
    pub hide_username: bool,
    #[prop_or_default]
    pub hide_videoid: bool,
}

#[function_component]
fn DetailTableRow(props: &DetailTableRowProps) -> Html {
    let modal_controls: ModalRendererControls =
        use_context().expect("ModalRendererControls should be available");
    let settings_context: SettingsContext =
        use_context().expect("SettingsContext should be available");
    let settings = settings_context.settings();
    let user_context: UserContext = use_context().expect("UserContext should be available");
    let original_thumb_indicator = html! {
        <Icon r#type={IconType::Original} tooltip="This is the original video thumbnail" />
    };
    let thumb_caption = use_memo((props.details.clone(), props.index), |(details, index)| {
        let DetailSlice::Thumbnails(ref thumbs) = details else {
            return ThumbnailCaption::None;
        };
        let thumb = &thumbs[*index];
        if let Some(timestamp) = thumb.timestamp {
            ThumbnailCaption::Text(format!("{timestamp}").into())
        } else {
            ThumbnailCaption::Html(original_thumb_indicator.clone())
        }
    });
    let voting_modal_trigger = {
        let modal_controls = modal_controls.clone();
        use_callback(
            (props.details.clone(), props.index),
            move |_: MouseEvent, (details, index)| {
                let detail = match details {
                    DetailSlice::Titles(ref list) => VotingDetail::Title(list[*index].clone()),
                    DetailSlice::Thumbnails(ref list) => {
                        VotingDetail::Thumbnail(list[*index].clone())
                    }
                };
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

    match props.details {
        DetailSlice::Titles(ref list) => {
            let t = &list[props.index];
            let expanded_layout = settings.title_table_layout == TableLayout::Expanded;
            let compressed_layout = settings.title_table_layout == TableLayout::Compressed;
            let rows = if compressed_layout { "1" } else { "2" };
            let title_column_classes =
                classes!("title-col", compressed_layout.then_some("compressed"));
            html! {
                <tr>
                    <td>{DateTime::from_timestamp_millis(t.time_submitted).map_or_else(|| t.time_submitted.to_string(), render_datetime)}</td>
                    if !props.hide_videoid {
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
                        {score_col!(title, t, expanded_layout)}
                    </td>
                    <td class="monospaced">{uuid_cell!(t.uuid, expanded_layout)}</td>
                    if !props.hide_username {
                        <td>{username_cell!(t.username, rows)}</td>
                    }
                    if !props.hide_userid {
                        <td>{userid_cell!(t.user_id, rows, expanded_layout)}</td>
                    }
                </tr>
            }
        }
        DetailSlice::Thumbnails(ref list) => {
            let t = &list[props.index];
            let expanded_layout = settings.thumbnail_table_layout == TableLayout::Expanded;
            let compressed_layout = settings.thumbnail_table_layout == TableLayout::Compressed;
            let rows = if compressed_layout { "1" } else { "2" };
            let render_thumbnails = settings.render_thumbnails_in_tables && expanded_layout;
            let onclick = {
                let list = list.clone();
                let index = props.index;
                Callback::from(move |_| {
                    let t = &list[index];
                    modal_controls.emit(ModalMessage::Open(html! {
                        <ThumbnailModal video_id={t.video_id.clone()} timestamp={t.timestamp} />
                    }));
                })
            };
            html! {
                <tr>
                    <td>{DateTime::from_timestamp_millis(t.time_submitted).map_or_else(|| t.time_submitted.to_string(), render_datetime)}</td>
                    if !props.hide_videoid {
                        <td class="monospaced"><YoutubeVideoLink videoid={t.video_id.clone()} multiline={expanded_layout} /></td>
                    }
                    if t.timestamp_missing {
                        <td><Icon r#type={IconType::TimestampMissing} tooltip="This thumbnail entry is missing a timestamp and cannot be rendered" /></td>
                    } else if render_thumbnails {
                        <Thumbnail video_id={t.video_id.clone()} timestamp={t.timestamp} caption={(*thumb_caption).clone()} container_type={ContainerType::td} />
                    } else {
                        <td {onclick} class="clickable">{t.timestamp.map_or(original_thumb_indicator, |ts| html! {{ts.to_string()}})}</td>
                    }
                    <td class={score_col_class} onclick={voting_modal_trigger}>
                        {score_col!(thumb, t, expanded_layout)}
                    </td>
                    <td class="monospaced">{uuid_cell!(t.uuid, expanded_layout)}</td>
                    if !props.hide_username {
                        <td>{username_cell!(t.username, rows)}</td>
                    }
                    if !props.hide_userid {
                        <td>{userid_cell!(t.user_id, rows, expanded_layout)}</td>
                    }
                </tr>
            }
        }
    }
}

#[function_component]
pub fn BaseDetailTableRenderer(props: &BaseDetailTableRendererProps) -> Html {
    let settings_context: SettingsContext =
        use_context().expect("SettingsContext should be available");
    let settings = settings_context.settings();
    let rendering_thumbnails = settings.render_thumbnails_in_tables
        && settings.thumbnail_table_layout == TableLayout::Expanded;
    let row_props = DetailTableRowProps {
        details: props.details.clone(),
        index: 0,
        hide_userid: props.hide_userid,
        hide_username: props.hide_username,
        hide_videoid: props.hide_videoid,
    };
    let header_classes = classes!("header", settings.sticky_headers.then_some("sticky"));
    match props.details {
        DetailSlice::Titles(ref list) => html! {
            <table class="detail-table titles" data-layout={AttrValue::Static(settings.title_table_layout.into())}>
                <tr class={header_classes}>
                    <th>{"Submitted"}</th>
                    if !props.hide_videoid {
                        <th>{"Video ID"}</th>
                    }
                    <th class="title-col">{"Title"}</th>
                    <th class="score-col">{"Score"}</th>
                    <th>{"UUID"}</th>
                    if !props.hide_username {
                        <th>{"Username"}</th>
                    }
                    if !props.hide_userid {
                        <th>{"User ID"}</th>
                    }
                </tr>
                { for list.iter().enumerate().map(|(i, t)| {
                    let mut row_props = row_props.clone();
                    row_props.index = i;
                    html! { <DetailTableRow key={t.uuid.clone()} ..row_props />}
                }) }
            </table>
        },
        DetailSlice::Thumbnails(ref list) => html! {
            <table class="detail-table thumbnails" data-layout={AttrValue::Static(settings.thumbnail_table_layout.into())}>
                <tr class={header_classes}>
                    <th>{"Submitted"}</th>
                    if !props.hide_videoid {
                        <th>{"Video ID"}</th>
                    }
                    if rendering_thumbnails {
                        <th class="thumbnail-header">{"Thumbnail"}</th>
                    } else {
                        <th>{"Timestamp"}</th>
                    }
                    <th class="score-col">{"Score"}</th>
                    <th>{"UUID"}</th>
                    if !props.hide_username {
                        <th>{"Username"}</th>
                    }
                    if !props.hide_userid {
                        <th>{"User ID"}</th>
                    }
                </tr>
                { for list.iter().enumerate().map(|(i, t)| {
                    let mut row_props = row_props.clone();
                    row_props.index = i;
                    html! { <DetailTableRow key={t.uuid.clone()} ..row_props />}
                }) }
            </table>
        },
    }
}

#[function_component]
pub fn UnpaginatedDetailTableRenderer(props: &DetailTableRendererProps) -> HtmlResult {
    let details = use_detail_download(props.url.clone(), props.mode, props.sort)?;
    let detail_slice = use_detail_slice((*details).as_ref().ok().cloned(), DetailIndex::All);

    if let Some(entry_count) = &props.entry_count {
        if let Ok(ref list) = *details {
            entry_count.set(Some(list.len()));
        } else {
            entry_count.set(None);
        }
    }

    if let Err(ref e) = *details {
        return Ok(html! {
            <center>
                <b>{"Failed to fetch details from the API :/"}</b>
                <pre>{format!("{e:?}")}</pre>
            </center>
        });
    }

    Ok(html! {
        <BaseDetailTableRenderer details={detail_slice} hide_videoid={props.hide_videoid} hide_userid={props.hide_userid} hide_username={props.hide_username} />
    })
}

#[function_component]
pub fn BasePaginatedDetailTableRenderer(props: &BaseDetailTableRendererProps) -> Html {
    let settings_context: SettingsContext =
        use_context().expect("SettingsContext should be available");
    let settings = settings_context.settings();
    let entries_per_page: usize = settings.entries_per_page.into();
    let state = use_location_state().get_state();
    let detail_slice = use_detail_slice(
        Some(props.details.clone()),
        DetailIndex::Page {
            size: entries_per_page,
            index: state.detail_table_page,
        },
    );

    let detail_count = props.details.len();
    let page_count = detail_count.div_ceil(entries_per_page);

    let inner_props = BaseDetailTableRendererProps {
        details: detail_slice,
        ..*props
    };

    html! {
        <>
            <BaseDetailTableRenderer ..{inner_props} />
            if page_count > 1 {
                <PageSelect {page_count} />
            }
        </>
    }
}

#[function_component]
pub fn PaginatedDetailTableRenderer(props: &DetailTableRendererProps) -> HtmlResult {
    let details = use_detail_download(props.url.clone(), props.mode, props.sort)?;
    let detail_slice = use_detail_slice((*details).as_ref().ok().cloned(), DetailIndex::All);

    if let Some(entry_count) = &props.entry_count {
        if let Ok(ref list) = *details {
            entry_count.set(Some(list.len()));
        } else {
            entry_count.set(None);
        }
    }

    if let Err(ref e) = *details {
        return Ok(html! {
            <center>
                <b>{"Failed to fetch details from the API :/"}</b>
                <pre>{format!("{e:?}")}</pre>
            </center>
        });
    }

    Ok(html! {
        <BasePaginatedDetailTableRenderer details={detail_slice} hide_videoid={props.hide_videoid} hide_userid={props.hide_userid} hide_username={props.hide_username} />
    })
}
