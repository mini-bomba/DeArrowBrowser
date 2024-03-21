use std::rc::Rc;

use yew::prelude::*;

use crate::{contexts::{WindowContext, StatusContext}, components::{detail_table::*, searchbar::Searchbar}, hooks::use_location_state};

#[function_component]
pub fn HomePage() -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let status_context: StatusContext = use_context().expect("StatusContext should be defined");
    let state = use_location_state().get_state();

    let mut url = match state.detail_table_mode {
        DetailType::Title => window_context.origin.join("/api/titles"),
        DetailType::Thumbnail => window_context.origin.join("/api/thumbnails"),
    }.expect("Should be able to create an API url");

    url.query_pairs_mut()
        .append_pair("offset", &format!("{}", state.detail_table_page*50))
        .append_pair("count", "50");

    let fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };

    let detail_count = status_context.map(|status_context| 
        match state.detail_table_mode {
            DetailType::Thumbnail => status_context.thumbnails,
            DetailType::Title     => status_context.titles,
        }
    );
    let page_count = detail_count.map(|detail_count| (detail_count+49)/50);
    
    html! {
        <>
            <div id="page-details">
                <Searchbar />
            </div>
            <TableModeSwitch entry_count={detail_count} />
            <Suspense {fallback}>
                <UnpaginatedDetailTableRenderer mode={state.detail_table_mode} url={Rc::new(url)} sort=false />
            </Suspense>
            if let Some(page_count) = page_count {
                <PageSelect  {page_count} />
            }
        </>
    }
}
