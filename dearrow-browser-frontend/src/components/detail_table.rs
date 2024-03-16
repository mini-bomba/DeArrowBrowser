use std::rc::Rc;

use chrono::NaiveDateTime;
use reqwest::Url;
use yew::{prelude::*, suspense::SuspensionResult};
use yew_router::prelude::*;
use dearrow_browser_api::*;

use crate::{pages::MainRoute, contexts::StatusContext, hooks::use_async_suspension, utils::render_naive_datetime};

#[derive(PartialEq, Clone, Copy)]
pub enum DetailType {
    Title,
    Thumbnail,
}

#[derive(Properties, PartialEq)]
pub struct TableModeSwitchProps {
    pub state: UseStateHandle<DetailType>,
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
            <span class="table-mode" onclick={set_titles_mode} selected={*props.state == DetailType::Title}>{"Titles"}</span>
            <span class="table-mode" onclick={set_thumbs_mode} selected={*props.state == DetailType::Thumbnail}>{"Thumbnails"}</span>
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

#[derive(Properties, PartialEq)]
pub struct DetailTableRendererProps {
    pub url: Rc<Url>,
    pub mode: DetailType,
    pub entry_count: Option<UseStateHandle<Option<usize>>>,
    #[prop_or_default]
    pub hide_userid: bool,
    #[prop_or_default]
    pub hide_username: bool,
    #[prop_or_default]
    pub hide_videoid: bool,
}

enum DetailList {
    Thumbnails(Vec<ApiThumbnail>),
    Titles(Vec<ApiTitle>),
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
pub fn DetailTableRenderer(props: &DetailTableRendererProps) -> HtmlResult {
    let status: StatusContext = use_context().expect("StatusResponse should be defined");
    let details = { 
        let result: SuspensionResult<Rc<Result<DetailList, anyhow::Error>>> = use_async_suspension(|(mode, url, _)| async move {
            let request = reqwest::get((*url).clone()).await?;
            let mut result = match mode {
                DetailType::Thumbnail => DetailList::Thumbnails(request.json().await?),
                DetailType::Title => DetailList::Titles(request.json().await?),
            };
            // Sort by time submited, most to least recent
            match result {
                DetailList::Thumbnails(ref mut list) => list.sort_unstable_by(|a, b| b.time_submitted.cmp(&a.time_submitted)),
                DetailList::Titles(ref mut list) => list.sort_unstable_by(|a, b| b.time_submitted.cmp(&a.time_submitted)),
            };
            Ok(result)
        }, (props.mode, props.url.clone(), status.map(|s| s.last_updated)));
        if let Some(count) = &props.entry_count {
            count.set(result.as_ref().ok().and_then(|r| r.as_ref().as_ref().ok()).map(|l| match l {
                DetailList::Thumbnails(list) => list.len(),
                DetailList::Titles(list) => list.len(),
            }));
        }
        result?
    };

    Ok(match *details {
        Err(ref e) => html! {
            <center>
                <b>{"Failed to fetch details from the API :/"}</b>
                <pre>{format!("{e:?}")}</pre>
            </center>
        },
        Ok(DetailList::Titles(ref list)) => html! {
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
                        <td>{NaiveDateTime::from_timestamp_millis(t.time_submitted).map_or(t.time_submitted.to_string(), render_naive_datetime)}</td>
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
        Ok(DetailList::Thumbnails(ref list)) => html! {
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
                        <td>{NaiveDateTime::from_timestamp_millis(t.time_submitted).map_or(t.time_submitted.to_string(), render_naive_datetime)}</td>
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
    })
}
