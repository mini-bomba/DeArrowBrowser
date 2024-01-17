use std::rc::Rc;
use chrono::NaiveDateTime;
use dearrow_browser_api::{StatusResponse, ApiThumbnail, ApiTitle, User};
use reqwest::Url;
use strum::IntoStaticStr;
use yew::{prelude::*, suspense::SuspensionResult};
use yew_hooks::{use_async_with_options, UseAsyncOptions, use_interval, use_title};
use yew_router::prelude::*;
use web_sys::{window, HtmlInputElement};
use gloo_console::error;

mod hooks;
mod utils;
use hooks::use_async_suspension;
use utils::*;


#[derive(Clone, Routable, PartialEq, IntoStaticStr)]
enum Route {
    #[at("/")]
    Home,
    #[at("/unverified")]
    Unverified,
    #[at("/video_id/:id")]
    Video { id: String },
    #[at("/user_id/:id")]
    User { id: String },
    #[at("/wip")]
    NotImplemented,
    #[not_found]
    #[at("/404")]
    NotFound,
}

#[derive(PartialEq, Clone, Copy)]
enum DetailType {
    Title,
    Thumbnail,
}

#[derive(Clone, PartialEq)]
struct WindowContext {
    origin: Url,
    logo_url: Option<AttrValue>,
}

#[derive(Clone, PartialEq)]
struct AppContext {
    last_updated: Option<i64>,
    last_modified: Option<i64>,
}

#[derive(Clone, Copy, PartialEq)]
struct UpdateClock(bool);

#[function_component]
fn App() -> Html {
    let window_context = use_memo((), |_| {
        let window = window().expect("window should exist");
        WindowContext {
            origin: Url::parse(
                window.location().origin().expect("window.location.origin should exist").as_str()
            ).expect("window.location.origin should be a valid URL"),
            logo_url: window.document()
                .and_then(|d| d.query_selector("link[rel=icon]").ok().flatten())
                .and_then(|el| el.get_attribute("href"))
                .map(AttrValue::from),
        }
    });
    let update_clock = use_state(|| UpdateClock(false));

    let status = {
        let window_context = window_context.clone();
        use_async_with_options::<_, StatusResponse, Rc<anyhow::Error>>(async move { 
            async { Ok(
                reqwest::get(window_context.origin.join("/api/status")?).await?
                    .json().await?
            )}.await.map_err(Rc::new)
        }, UseAsyncOptions::enable_auto())
    };
    {
        let status = status.clone();
        let update_clock = update_clock.clone();
        use_interval(move || {
            update_clock.set(UpdateClock(!update_clock.0));
            status.run();
        }, 60*1000);
    }
    let app_context = use_memo(
        status.data.as_ref().map(|d| (d.last_updated, d.last_modified)).unzip(),
        |&data| {
            let (last_updated, last_modified) = data;
            AppContext {
                last_updated,
                last_modified,
            }
        }
    );

    html! {
        <ContextProvider<Rc<WindowContext>> context={window_context}>
        <ContextProvider<Rc<AppContext>> context={app_context}>
        <ContextProvider<UpdateClock> context={*update_clock}>
            <BrowserRouter>
                <Switch<Route> render={render_route} />
            </BrowserRouter>
        </ContextProvider<UpdateClock>>
        </ContextProvider<Rc<AppContext>>>
        </ContextProvider<Rc<WindowContext>>>
    }
}

macro_rules! search_block {
    ($id:expr, $name:expr, $callback:expr) => {
        html! {
            <div>
                <label for={$id} >{concat!("Search by ", $name)}</label>
                <input id={$id} placeholder={$name} onkeydown={$callback} value="" />
            </div>
        }
    };
}

#[function_component]
fn Searchbar() -> Html {
    let navigator = use_navigator().expect("navigator should exist");

    let uuid_search = {
        let navigator = navigator.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                navigator.push(&Route::NotImplemented);
            }
        })
    };
    let uid_search = {
        let navigator = navigator.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                let input: HtmlInputElement = e.target_unchecked_into();
                navigator.push(&Route::User {id: input.value()});
            }
        })
    };
    let vid_search = { 
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                let input: HtmlInputElement = e.target_unchecked_into();
                let value = input.value();
                navigator.push(&Route::Video {
                    id: if let Ok(url) = Url::parse(&value) {  // Try to parse as URL
                        url.query_pairs().find(|(ref k, _)| k == "v").map(|(_, v)| v.to_string()).or_else(||  // Try to find a "v" query param
                            url.path_segments().and_then(|it| it.filter(|s| !s.is_empty()).last()).map(ToString::to_string)  // Fall back to last non-empty path segment if none found
                        ).unwrap_or(value)  // Fall back to original value
                    } else {
                        value  // Fall back to original value
                    }
                });
            }
        })
    };

    html! {
        <div id="searchbar">
            {search_block!("uuid_search", "UUID", uuid_search)}
            {search_block!("vid_search", "Video ID", vid_search)}
            {search_block!("uid_search", "User ID", uid_search)}
        </div>
    }
}

#[function_component]
fn Header() -> Html {
    let navigator = use_navigator().expect("navigator should exist");
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");

    let go_home = {
        Callback::from(move |_| {
            navigator.push(&Route::Home);
        })
    };

    html! {
        <div id="header">
            if let Some(url) = &window_context.logo_url {
                <img src={url} class="clickable" onclick={go_home.clone()} />
            }
            <div>
                <h1 class="clickable" onclick={go_home}>{"DeArrow Browser"}</h1>
            </div>
        </div>
    }
}

#[function_component]
fn Footer() -> Html {
    let app_context: Rc<AppContext> = use_context().expect("AppContext should be defined");
    let _ = use_context::<UpdateClock>();

    let last_updated = match app_context.last_updated.and_then(NaiveDateTime::from_timestamp_millis).map(|dt| dt.and_utc()) {
        None => AttrValue::from("..."),
        Some(time) => AttrValue::from(render_datetime_with_delta(time)),
    };
    let last_modified = match app_context.last_modified.and_then(NaiveDateTime::from_timestamp_millis).map(|dt| dt.and_utc()) {
        None => AttrValue::from("..."),
        Some(time) => AttrValue::from(render_datetime_with_delta(time)),
    };

    html! {
        <div id="footer">
            <table>
                <tr>
                    <td>{"Last update:"}</td>
                    <td>{last_updated}</td>
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
                        {" © mini_bomba 2024, licensed under "}
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

fn render_route(route: Route) -> Html {
    let document = window().expect("window should exist")
        .document().expect("document should exist");
    document.set_title(match &route {
        Route::Home => "DeArrow Browser".to_string(),
        Route::Unverified => "Unverified titles - DeArrow Browser".to_string(),
        Route::NotFound => "Page not found - DeArrow Browser".to_string(),
        Route::NotImplemented => "Not implemented - DeArrow Browser".to_string(),
        Route::Video { ref id } => format!("VideoID {id} - DeArrow Browser"),
        Route::User { ref id } => format!("UserID {id} - Dearrow Browser"),
    }.as_str());
    let route_html = match route {
        Route::Home => html! {<HomePage/>},
        Route::Unverified => html! {<UnverifiedPage/>},
        Route::Video { ref id } => html! {<VideoPage videoid={id.clone()} />},
        Route::User { ref id } => html! {<UserPage userid={id.clone()} />},
        Route::NotFound => html! {
            <>
                <h2>{"404 - Not found"}</h2>
                <h3>{"Looks like you've entered an invalid URL"}</h3>
                <Link<Route> to={Route::Home}>{"Return to home page"}</Link<Route>>
            </>
        },
        Route::NotImplemented => html! {
            <>
                <h2>{"Not implemented"}</h2>
                <h3>{"This feature is not implemented yet"}</h3>
                <Link<Route> to={Route::Home}>{"Return to home page"}</Link<Route>>
            </>
        },
    };
    let route_name: &'static str = (&route).into();
    html! {
        <>
            <Header />
            <div id="content" data-route={route_name}>
                {route_html}
            </div>
            <Footer />
        </>
    }
}

#[derive(Properties, PartialEq)]
struct TableModeSwitchProps {
    state: UseStateHandle<DetailType>,
    entry_count: Option<usize>,
}

#[function_component]
fn TableModeSwitch(props: &TableModeSwitchProps) -> Html {
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
struct DetailTableRendererProps {
    url: Rc<Url>,
    mode: DetailType,
    entry_count: Option<UseStateHandle<Option<usize>>>,
    #[prop_or_default]
    hide_userid: bool,
    #[prop_or_default]
    hide_username: bool,
    #[prop_or_default]
    hide_videoid: bool,
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
                    <Link<Route> to={Route::Video { id: $videoid.to_string() }}>{"🔍"}</Link<Route>>
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
                    <Link<Route> to={Route::User { id: $userid.to_string() }}>{"🔍"}</Link<Route>>
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
fn DetailTableRenderer(props: &DetailTableRendererProps) -> HtmlResult {
    let app_context: Rc<AppContext> = use_context().expect("AppContext should be defined");
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
        }, (props.mode, props.url.clone(), app_context.last_updated));
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
                        <td class="score-col">{title_score(t)}<br />{title_flags(t)}</td>
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
                        <td class="score-col">{thumb_score(t)}<br />{thumbnail_flags(t)}</td>
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

#[function_component]
fn HomePage() -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let table_mode = use_state_eq(|| DetailType::Title);
    let entry_count = use_state_eq(|| None);

    let url = match *table_mode {
        DetailType::Title => window_context.origin.join("/api/titles"),
        DetailType::Thumbnail => window_context.origin.join("/api/thumbnails"),
    }.expect("Should be able to create an API url");

    let fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };
    
    html! {
        <>
            <div id="page-details">
                <Searchbar />
            </div>
            <TableModeSwitch state={table_mode.clone()} entry_count={*entry_count} />
            <Suspense {fallback}>
                <DetailTableRenderer mode={*table_mode} url={Rc::new(url)} {entry_count} />
            </Suspense>
        </>
    }
}

#[function_component]
fn UnverifiedPage() -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let entry_count = use_state_eq(|| None);

    let url = window_context.origin.join("/api/titles/unverified").expect("Should be able to create an API url");

    let fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };
    
    html! {
        <>
            if let Some(count) = *entry_count {
                <span>
                    if count == 1 {
                        {"1 entry"}
                    } else {
                        {format!("{count} entries")}
                    }
                </span>
            }
            <Suspense {fallback}>
                <DetailTableRenderer mode={DetailType::Title} url={Rc::new(url)} {entry_count} />
            </Suspense>
        </>
    }
}

#[derive(Properties, PartialEq)]
struct OriginalTitleProps {
    videoid: AttrValue,
}

#[function_component]
fn OriginalTitle(props: &OriginalTitleProps) -> HtmlResult {
    let title = use_async_suspension(|vid| async move {
        let result = utils::get_original_title(vid.to_string()).await;
        if let Err(ref e) = result {
            error!(format!("Failed to fetch original title for video {vid}: {e:?}"));
        }
        result
    }, props.videoid.clone())?;
    if let Ok(ref t) = *title {
        Ok(html!{<span>{t.as_str()}</span>})
    } else {
        Ok(html!{<span><em>{"Failed to fetch original title"}</em></span>})
    }
}

#[derive(Properties, PartialEq)]
struct VideoDetailsTableProps {
    videoid: AttrValue,
    mode: DetailType,
}

#[function_component]
fn VideoDetailsTable(props: &VideoDetailsTableProps) -> Html {
    let fallback = html!{
        <span><em>{"Loading..."}</em></span>
    };
    html! {
        <div id="details-table">
            <div>{format!("Video ID: {}", props.videoid)}</div>
            <div hidden={props.mode != DetailType::Title}>
                {"Original title: "}
                <Suspense {fallback}><OriginalTitle videoid={props.videoid.clone()} /></Suspense>
            </div>
            <div><a href={format!("https://youtu.be/{}", props.videoid)}>{"View on YouTube"}</a></div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct VideoPageProps {
    videoid: AttrValue,
}

#[function_component]
fn VideoPage(props: &VideoPageProps) -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let table_mode = use_state_eq(|| DetailType::Title);
    let entry_count = use_state_eq(|| None);

    let url = match *table_mode {
        DetailType::Title => window_context.origin.join(format!("/api/titles/video_id/{}", props.videoid).as_str()),
        DetailType::Thumbnail => window_context.origin.join(format!("/api/thumbnails/video_id/{}", props.videoid).as_str()),
    }.expect("Should be able to create an API url");

    let fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };
    
    html! {
        <>
            <div id="page-details">
                <iframe src={format!("https://www.youtube-nocookie.com/embed/{}", props.videoid)} allowfullscreen=true />
                <VideoDetailsTable videoid={props.videoid.clone()} mode={*table_mode} />
            </div>
            <TableModeSwitch state={table_mode.clone()} entry_count={*entry_count} />
            <Suspense {fallback}>
                <DetailTableRenderer mode={*table_mode} url={Rc::new(url)} {entry_count} hide_videoid=true />
            </Suspense>
        </>
    }
}

#[derive(Properties, PartialEq)]
struct UserDetailsProps {
    userid: AttrValue,
}

#[function_component]
fn UserDetails(props: &UserDetailsProps) -> HtmlResult {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let app_context: Rc<AppContext> = use_context().expect("AppContext should be defined");
    let url = window_context.origin.join(format!("/api/users/user_id/{}", props.userid).as_str()).expect("Should be able to create an API url");
    let result: Rc<Result<User, anyhow::Error>> = use_async_suspension(|(url, _)| async move {
        Ok(reqwest::get((url).clone()).await?.json().await?)
    }, (url, app_context.last_updated))?;

    Ok(match *result {
        Ok(ref user) => html! {
            <>
                <div>{format!("UserID: {}", props.userid.clone())}
                if user.vip {
                   <span title="This user is a VIP">{"👑"}</span> 
                }
                </div> 
                <div>
                if let Some(username) = &user.username {
                    {format!("Username: {username}")}
                } else {
                    {"Username: "}<em>{"No username set"}</em>
                }
                if user.username_locked {
                    <span title="This user's username is locked">{"🔒"}</span>
                }
                </div>
                <div>{format!("Titles: {}", user.title_count)}</div>
                <div>{format!("Thumbnails: {}", user.thumbnail_count)}</div>
            </>
        },
        Err(ref e) => html! {
            <div>{"Failed to fetch user data"}<br/><pre>{format!("{e:?}")}</pre></div>
        }
    })
}

#[derive(Properties, PartialEq)]
struct UserPageProps {
    userid: AttrValue,
}

#[function_component]
fn UserPage(props: &UserPageProps) -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let table_mode = use_state_eq(|| DetailType::Title);
    let entry_count = use_state_eq(|| None);

    let url = match *table_mode {
        DetailType::Title => window_context.origin.join(format!("/api/titles/user_id/{}", props.userid).as_str()),
        DetailType::Thumbnail => window_context.origin.join(format!("/api/thumbnails/user_id/{}", props.userid).as_str()),
    }.expect("Should be able to create an API url");

    let details_fallback = html! {
        <div><b>{"Loading..."}</b></div>
    };
    let table_fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };
    
    html! {
        <>
            <div id="page-details">
                <div id="details-table">
                    <Suspense fallback={details_fallback}><UserDetails userid={props.userid.clone()} /></Suspense>
                </div>
            </div>
            <TableModeSwitch state={table_mode.clone()} entry_count={*entry_count} />
            <Suspense fallback={table_fallback}>
                <DetailTableRenderer mode={*table_mode} url={Rc::new(url)} {entry_count} hide_userid=true hide_username=true />
            </Suspense>
        </>
    }
}


fn main() {
    yew::Renderer::<App>::new().render();
}
