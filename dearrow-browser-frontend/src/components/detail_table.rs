use std::rc::Rc;
use std::str::FromStr;

use chrono::DateTime;
use reqwest::Url;
use web_sys::HtmlInputElement;
use yew::{prelude::*, suspense::SuspensionResult};
use yew_router::prelude::*;
use dearrow_browser_api::*;

use crate::{pages::MainRoute, contexts::StatusContext, hooks::{use_async_suspension, use_memo_state_eq}, utils::{render_datetime, RcEq}};

#[derive(PartialEq, Clone, Copy)]
pub enum DetailType {
    Title,
    Thumbnail,
}

#[derive(Properties, PartialEq)]
pub struct TableModeSwitchProps {
    pub state: UseStateHandle<DetailType>,
    #[prop_or_default]
    pub entry_count: Option<usize>,
}

#[function_component]
pub fn TableModeSwitch(props: &TableModeSwitchProps) -> Html {
    let set_titles_mode = {
        let state = props.state.clone();
        Callback::from(move |_| {
            state.set(DetailType::Title);
        })
    };
    let set_thumbs_mode = {
        let state = props.state.clone();
        Callback::from(move |_| {
            state.set(DetailType::Thumbnail);
        })
    };

    html! {
        <div class="table-mode-switch">
            <span class="table-mode button" onclick={set_titles_mode} selected={*props.state == DetailType::Title}>{"Titles"}</span>
            <span class="table-mode button" onclick={set_thumbs_mode} selected={*props.state == DetailType::Thumbnail}>{"Thumbnails"}</span>
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
    pub state: UseStateHandle<usize>,
    pub page_count: usize,
}

#[function_component]
pub fn PageSelect(props: &PageSelectProps) -> Html {
    let prev_page = {
        let state = props.state.clone();
        Callback::from(move |_| {
            state.set(state.checked_sub(1).unwrap_or(0));
        })
    };
    let next_page = {
        let state = props.state.clone();
        let max_page = props.page_count-1;
        Callback::from(move |_| {
            state.set(max_page.min(*state+1));
        })
    };
    let input_changed = {
        let state = props.state.clone();
        let page_count = props.page_count;
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            match usize::from_str(&input.value()) {
                Err(_) => {},
                Ok(new_page) => state.set(new_page.clamp(1,page_count)-1),
            };
            input.set_value(&format!("{}", *state+1));
        })
    };

    html! {
        <div class="page-select">
            <div class="button" onclick={prev_page}>{"prev"}</div>
            <div>
                {"page"}
                <input type="number" min=1 max={format!("{}", props.page_count)} ~value={format!("{}", *props.state+1)} onchange={input_changed} />
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
            if title.votes - title.downvotes < -1 {
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
            if title.removed {
                <span title="This title was removed by a VIP">{"❌"}</span>
            }
            if title.vip {
                <span title="This title was submitted by a VIP">{"👑"}</span>
            }
            if title.shadow_hidden {
                <span title="This title is shadowhidden">{"🚫"}</span>
            }
        </>
    }
}

fn thumbnail_flags(thumb: &ApiThumbnail) -> Html {
    html! {
        <>
            if thumb.votes - thumb.downvotes < -1 {
                <span title="This thumbnail was removed by the downvotes">{"👎"}</span>
            } else if thumb.score < 0 {
                <span title="This thumbnail should only appear in submission menus (score below 0)" class="grayscale">{"👎"}</span>
            }
            if thumb.locked {
                <span title="This thumbnail was locked by a VIP">{"🔒"}</span>
            }
            if thumb.removed {
                <span title="This thumbnail was removed by a VIP">{"❌"}</span>
            }
            if thumb.vip {
                <span title="This thumbnail was submitted by a VIP">{"👑"}</span>
            }
            if thumb.shadow_hidden {
                <span title="This thumbnail is shadowhidden">{"🚫"}</span>
            }
        </>
    }
}

fn title_score(title: &ApiTitle) -> Html {
    html! {
        <span class="hoverswitch">
            <span>{title.score}</span>
            <span>{format!("👍 {} | {} 👎", title.votes, title.downvotes)}</span>
        </span>
    }
}
fn thumb_score(thumb: &ApiThumbnail) -> Html {
    html! {
        <span class="hoverswitch">
            <span>{thumb.score}</span>
            <span>{format!("👍 {} | {} 👎", thumb.votes, thumb.downvotes)}</span>
        </span>
    }
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

macro_rules! video_link {
    ($videoid:expr) => {
        html! {
            <>
                <a href={format!("https://youtu.be/{}", $videoid)} title="View this video on YouTube" target="_blank">{$videoid.clone()}</a><br />
                <span class="icon-link" title="View this video in DeArrow Browser">
                    <Link<MainRoute> to={MainRoute::Video { id: $videoid.to_string() }}>{"🔍"}</Link<MainRoute>>
                </span>
            </>
        }
    };
}

macro_rules! user_link {
    ($userid:expr) => {
        html! {
            <>
                <textarea readonly=true ~value={$userid.to_string()} /><br />
                <span class="icon-link" title="View this user in DeArrow Browser">
                    <Link<MainRoute> to={MainRoute::User { id: $userid.to_string() }}>{"🔍"}</Link<MainRoute>>
                </span>
            </>
        }
    };
}

macro_rules! username_link {
    ($username:expr) => {
        if let Some(ref name) = $username {
            html! {<textarea readonly=true ~value={name.to_string()} />}
        } else {
            html! {{"-"}}
        }
    };
}

#[function_component]
pub fn BaseDetailTableRenderer(props: &BaseDetailTableRendererProps) -> Html{
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
                { for list.iter().map(|t| html! {
                    <tr key={&*t.uuid}>
                        <td>{DateTime::from_timestamp_millis(t.time_submitted).map_or(t.time_submitted.to_string(), render_datetime)}</td>
                        if !props.hide_videoid {
                            <td>{video_link!(t.video_id)}</td>
                        }
                        <td class="title-col">{t.title.clone()}<br />{original_indicator!(t.original, title)}</td>
                        <td class="score-col hoverswitch-trigger">{title_score(t)}<br />{title_flags(t)}</td>
                        <td>{t.uuid.clone()}</td>
                        if !props.hide_username {
                            <td>{username_link!(t.username)}</td>
                        }
                        if !props.hide_userid {
                            <td>{user_link!(t.user_id)}</td>
                        }
                    </tr>
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
                { for list.iter().map(|t| html! {
                    <tr key={&*t.uuid}>
                        <td>{DateTime::from_timestamp_millis(t.time_submitted).map_or(t.time_submitted.to_string(), render_datetime)}</td>
                        if !props.hide_videoid {
                            <td>{video_link!(t.video_id)}</td>
                        }
                        <td>{t.timestamp.map_or(original_indicator!(t.original, thumbnail), |ts| html! {{ts.to_string()}})}</td>
                        <td class="score-col hoverswitch-trigger">{thumb_score(t)}<br />{thumbnail_flags(t)}</td>
                        <td>{t.uuid.clone()}</td>
                        if !props.hide_username {
                            <td>{username_link!(t.username)}</td>
                        }
                        if !props.hide_userid {
                            <td>{user_link!(t.user_id)}</td>
                        }
                    </tr>
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
    const PAGE_SIZE: usize = 50;
    let current_page = use_memo_state_eq(props.mode, || 0);
    let details = use_detail_download(props.url.clone(), props.mode, props.sort)?;
    let detail_slice = use_detail_slice(details.clone(), DetailIndex::Page { size: PAGE_SIZE, index: *current_page });

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
    let page_count = (detail_count+PAGE_SIZE-1)/PAGE_SIZE;
    
    Ok(html! {
        <>
            <BaseDetailTableRenderer details={detail_slice} hide_videoid={props.hide_videoid} hide_userid={props.hide_userid} hide_username={props.hide_username} />
            if page_count > 1 {
                <PageSelect state={current_page} {page_count} />
            }
        </>
    })
}
