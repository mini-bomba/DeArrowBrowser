use std::rc::Rc;

use gloo_console::error;
use yew::prelude::*;

use crate::{utils, components::detail_table::*, hooks::{use_async_suspension, use_location_state}, contexts::WindowContext};

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
pub struct VideoPageProps {
    pub videoid: AttrValue,
}

#[function_component]
pub fn VideoPage(props: &VideoPageProps) -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let state = use_location_state().get_state();
    // let table_mode = use_state_eq(|| DetailType::Title);
    let entry_count = use_state_eq(|| None);

    let url = match state.detail_table_mode {
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
                <VideoDetailsTable videoid={props.videoid.clone()} mode={state.detail_table_mode} />
            </div>
            <TableModeSwitch entry_count={*entry_count} />
            <Suspense {fallback}>
                <PaginatedDetailTableRenderer mode={state.detail_table_mode} url={Rc::new(url)} {entry_count} hide_videoid=true />
            </Suspense>
        </>
    }
}
