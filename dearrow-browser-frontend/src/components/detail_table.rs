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
use std::str::FromStr;

use chrono::DateTime;
use reqwest::Url;
use web_sys::HtmlInputElement;
use yew::{prelude::*, suspense::SuspensionResult};
use dearrow_browser_api::unsync::*;

use crate::{components::{links::*, modals::{thumbnail::ThumbnailModal, ModalMessage}, youtube::YoutubeVideoLink}, contexts::{SettingsContext, StatusContext}, hooks::{use_async_suspension, use_location_state}, pages::LocationState, settings::TableLayout, utils::{render_datetime, RcEq}, ModalRendererControls};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum DetailType {
    Title,
    Thumbnail,
}

impl Default for DetailType {
    fn default() -> Self {
        Self::Title
    }
}

#[derive(Properties, PartialEq)]
pub struct TableModeSwitchProps {
    #[prop_or_default]
    pub entry_count: Option<usize>,
}

#[function_component]
pub fn TableModeSwitch(props: &TableModeSwitchProps) -> Html {
    let state_handle = use_location_state();
    let state = state_handle.get_state();

    let set_titles_mode = {
        let state_handle = state_handle.clone();
        Callback::from(move |_| {
            state_handle.push_state(LocationState {
                detail_table_mode: DetailType::Title,
                detail_table_page: 0,
            });
        })
    };
    let set_thumbs_mode = {
        Callback::from(move |_| {
            state_handle.push_state(LocationState {
                detail_table_mode: DetailType::Thumbnail,
                detail_table_page: 0,
            });
        })
    };

    html! {
        <div class="table-mode-switch">
            <span class="table-mode button" onclick={set_titles_mode} selected={state.detail_table_mode == DetailType::Title}>{"Titles"}</span>
            <span class="table-mode button" onclick={set_thumbs_mode} selected={state.detail_table_mode == DetailType::Thumbnail}>{"Thumbnails"}</span>
            if let Some(count) = props.entry_count {
                <span>
                    if count == 1 {
                        {"1 entry"}
                    } else {
                        {format!("{count} entries")}
                    }
                </span>
            }
        </div>
    }
    
}

#[derive(Properties, PartialEq, Clone)]
pub struct PageSelectProps {
    // pub state: UseStateHandle<usize>,
    pub page_count: usize,
}

#[function_component]
pub fn PageSelect(props: &PageSelectProps) -> Html {
    let state_handle = use_location_state();
    let state = state_handle.get_state();

    let prev_page = {
        let state_handle = state_handle.clone();
        Callback::from(move |_| {
            let mut state = state;
            state.detail_table_page = state.detail_table_page.saturating_sub(1);
            state_handle.replace_state(state);
        })
    };
    let next_page = {
        let state_handle = state_handle.clone();
        let max_page = props.page_count-1;
        Callback::from(move |_| {
            let mut state = state;
            state.detail_table_page = max_page.min(state.detail_table_page+1);
            state_handle.replace_state(state);
        })
    };
    let input_changed = {
        let state_handle = state_handle.clone();
        let page_count = props.page_count;
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut state = state;
            match usize::from_str(&input.value()) {
                Err(_) => {},
                Ok(new_page) => {
                    state.detail_table_page = new_page.clamp(1,page_count)-1;
                    state_handle.replace_state(state);
                },
            };
            input.set_value(&format!("{}", state.detail_table_page+1));
        })
    };

    html! {
        <div class="page-select">
            <div class="button" onclick={prev_page}>{"prev"}</div>
            <div>
                {"page"}
                <input type="number" min=1 max={format!("{}", props.page_count)} ~value={format!("{}", state.detail_table_page+1)} onchange={input_changed} />
                {format!("/{}", props.page_count)}
            </div>
            <div class="button" onclick={next_page}>{"next"}</div>
        </div>
    }
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
pub fn use_detail_download(url: Rc<Url>, mode: DetailType, sort: bool) -> SuspensionResult<Rc<Result<DetailList, anyhow::Error>>> {
    let status: StatusContext = use_context().expect("StatusResponse should be defined");
    use_async_suspension(|(mode, url, sort, _)| async move {
        let request = reqwest::get((*url).clone()).await?;
        let mut result = match mode {
            DetailType::Thumbnail => DetailList::Thumbnails(request.json().await?),
            DetailType::Title => DetailList::Titles(request.json().await?),
        };
        if sort {
        // Sort by time submited, most to least recent
            match result {
                DetailList::Thumbnails(ref mut list) => list.sort_unstable_by(|a, b| b.time_submitted.cmp(&a.time_submitted)),
                DetailList::Titles(ref mut list) => list.sort_unstable_by(|a, b| b.time_submitted.cmp(&a.time_submitted)),
            }
        }
        Ok(result)
    }, (mode, url, sort, status.map(|s| s.last_updated)))
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
    pub entry_count: Option<UseStateHandle<Option<usize>>>,
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
    Thumbnails(Rc<[ApiThumbnail]>),
    Titles(Rc<[ApiTitle]>),
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
    Page {
        size: usize,
        index: usize,
    },
    All,
}

#[hook]
pub fn use_detail_slice(details: Rc<Result<DetailList, anyhow::Error>>, index: DetailIndex) -> DetailSlice {
    (*use_memo((RcEq(details), index), |(details, index)|
        match (&**details, index) {
            (Err(_), _)                                                                 
                => DetailSlice::Thumbnails(Rc::new([])), // dummy slice on error

            (Ok(DetailList::Thumbnails(ref thumbs)), DetailIndex::Page { size, index }) 
                => DetailSlice::Thumbnails(if size*(index+1) > thumbs.len() {thumbs.get(size*index..)} else {thumbs.get(size*index..size*(index+1))}.unwrap_or(&[]).into()),
            (Ok(DetailList::Thumbnails(ref thumbs)), DetailIndex::All)                  
                => DetailSlice::Thumbnails((&**thumbs).into()),

            (Ok(DetailList::Titles(ref titles)), DetailIndex::Page { size, index })     
                => DetailSlice::Titles(if size*(index+1) > titles.len() {titles.get(size*index..)} else {titles.get(size*index..size*(index+1))}.unwrap_or(&[]).into()),
            (Ok(DetailList::Titles(ref titles)), DetailIndex::All)                      
                => DetailSlice::Titles((&**titles).into()),
        }
    )).clone()
}

fn title_flags(title: &ApiTitle) -> Html {
    html! {
        <>
            if title.removed || title.shadow_hidden {
                if title.removed {
                    <span title="This title was removed by a VIP">{"❌"}</span>
                }
                if title.shadow_hidden {
                    <span title="This title is shadowhidden">{"🚫"}</span>
                }
            } else if title.votes - title.downvotes < -1 {
                <span title="This title was removed by the downvotes">{"👎"}</span>
            } else if title.votes < 0 {
                <span title="This title was replaced by the submitter">{"👎"}</span>
            } else if title.score < 0 {
                <span title="This title should only appear in submission menus (score below 0)" class="grayscale">{"👎"}</span>
            }
            if title.unverified {
                <span title="This title was submitted by an unverified user (-1 score)">{"❓"}</span>
            }
            if title.locked {
                <span title="This title was locked by a VIP">{"🔒"}</span>
            }
            if title.vip {
                <span title="This title was submitted by a VIP">{"👑"}</span>
            }
        </>
    }
}

fn thumbnail_flags(thumb: &ApiThumbnail) -> Html {
    html! {
        <>
            if thumb.removed || thumb.shadow_hidden {
                if thumb.removed {
                    <span title="This thumbnail was removed by a VIP">{"❌"}</span>
                }
                if thumb.shadow_hidden {
                    <span title="This thumbnail is shadowhidden">{"🚫"}</span>
                }
            } else if thumb.votes - thumb.downvotes < -1 {
                <span title="This thumbnail was removed by the downvotes">{"👎"}</span>
            } else if thumb.score < 0 {
                <span title="This thumbnail should only appear in submission menus (score below 0)" class="grayscale">{"👎"}</span>
            }
            if thumb.locked {
                <span title="This thumbnail was locked by a VIP">{"🔒"}</span>
            }
            if thumb.vip {
                <span title="This thumbnail was submitted by a VIP">{"👑"}</span>
            }
        </>
    }
}

/// Stupid recursive function for counting elements in a `VNode` tree
fn html_length(html: &Html) -> usize {
    match html {
        Html::VList(ref list) => list.iter().map(html_length).sum(),
        _ => 1,
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
        if $expanded {
            html! {
                <>
                    <span class="hoverswitch">
                        <span>{$detail.score}</span>
                        <span>{format!("👍 {} | {} 👎", $detail.votes, $detail.downvotes)}</span>
                    </span>
                    <br />
                    {detail_flags!($type, $detail)}
                </>
            }
        } else {
            html! {
                <>
                    {format!("{} | 👍 {} | {} 👎", $detail.score, $detail.votes, $detail.downvotes)}
                    {bar_prepender_if_not_empty(detail_flags!($type, $detail))}
                </>
            }
        }
    };
}

macro_rules! original_indicator {
    ($original:expr, $detail_name:expr) => {
        if $original {
            html! {
                <span title={stringify!(This is the original video $detail_name)}>{"♻️"}</span>
            }
        } else {
            html! {}
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
    let modal_controls: ModalRendererControls = use_context().expect("ModalRendererControls should be available");
    let settings_context: SettingsContext = use_context().expect("SettingsContext should be available");
    let settings = settings_context.settings();

    match props.details {
        DetailSlice::Titles(ref list) => {
            let t = &list[props.index];
            let expanded_layout = settings.title_table_layout == TableLayout::Expanded;
            let compressed_layout = settings.title_table_layout == TableLayout::Compressed;
            let rows = if compressed_layout { "1" } else { "2" }; 
            let title_column_classes = classes!("title-col", compressed_layout.then_some("compressed"));
            html! {
                <tr>
                    <td>{DateTime::from_timestamp_millis(t.time_submitted).map_or(t.time_submitted.to_string(), render_datetime)}</td>
                    if !props.hide_videoid {
                        <td class="monospaced"><YoutubeVideoLink videoid={t.video_id.clone()} multiline={expanded_layout} /></td>
                    }
                    <td class={title_column_classes}>
                        {t.title.clone()}
                        if expanded_layout { <br /> } else {{""}}
                        {original_indicator!(t.original, title)}
                    </td>
                    <td class="score-col hoverswitch-trigger">
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
        },
        DetailSlice::Thumbnails(ref list) => {
            let t = &list[props.index];
            let expanded_layout = settings.thumbnail_table_layout == TableLayout::Expanded;
            let compressed_layout = settings.thumbnail_table_layout == TableLayout::Compressed;
            let rows = if compressed_layout { "1" } else { "2" }; 
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
                    <td>{DateTime::from_timestamp_millis(t.time_submitted).map_or(t.time_submitted.to_string(), render_datetime)}</td>
                    if !props.hide_videoid {
                        <td class="monospaced"><YoutubeVideoLink videoid={t.video_id.clone()} multiline={expanded_layout} /></td>
                    }
                    <td {onclick} class="clickable">{t.timestamp.map_or(original_indicator!(t.original, thumbnail), |ts| html! {{ts.to_string()}})}</td>
                    <td class="score-col hoverswitch-trigger">
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
    let row_props = DetailTableRowProps {
        details: props.details.clone(),
        index: 0,
        hide_userid: props.hide_userid,
        hide_username: props.hide_username,
        hide_videoid: props.hide_videoid,
    };
    match props.details {
        DetailSlice::Titles(ref list) => html! {
            <table class="detail-table titles">
                <tr class="header">
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
            <table class="detail-table thumbnails">
                <tr class="header">
                    <th>{"Submitted"}</th>
                    if !props.hide_videoid {
                        <th>{"Video ID"}</th>
                    }
                    <th>{"Timestamp"}</th>
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
    let detail_slice = use_detail_slice(details.clone(), DetailIndex::All);

    if let Some(entry_count) = &props.entry_count {
        if let Ok(ref list) = *details {
            entry_count.set(Some(list.len()));
        } else {
            entry_count.set(None);
        }
    }

    if let Err(ref e) = *details {
        return Ok(html!{
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
pub fn PaginatedDetailTableRenderer(props: &DetailTableRendererProps) -> HtmlResult {
    let settings_context: SettingsContext = use_context().expect("SettingsContext should be available");
    let settings = settings_context.settings();
    let entries_per_page: usize = settings.entries_per_page.into();
    let state = use_location_state().get_state();
    let details = use_detail_download(props.url.clone(), props.mode, props.sort)?;
    let detail_slice = use_detail_slice(details.clone(), DetailIndex::Page { size: entries_per_page, index: state.detail_table_page });

    if let Some(entry_count) = &props.entry_count {
        if let Ok(ref list) = *details {
            entry_count.set(Some(list.len()));
        } else {
            entry_count.set(None);
        }
    }

    if let Err(ref e) = *details {
        return Ok(html!{
            <center>
                <b>{"Failed to fetch details from the API :/"}</b>
                <pre>{format!("{e:?}")}</pre>
            </center>
        });
    }
    let detail_count = details.as_ref().as_ref().unwrap().len();
    let page_count = (detail_count+entries_per_page)/entries_per_page;
    
    Ok(html! {
        <>
            <BaseDetailTableRenderer details={detail_slice} hide_videoid={props.hide_videoid} hide_userid={props.hide_userid} hide_username={props.hide_username} />
            if page_count > 1 {
                <PageSelect {page_count} />
            }
        </>
    })
}
