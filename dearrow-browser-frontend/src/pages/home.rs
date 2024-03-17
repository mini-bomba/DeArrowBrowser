use std::rc::Rc;

use yew::prelude::*;

use crate::{contexts::WindowContext, components::{detail_table::*, searchbar::Searchbar}};

#[function_component]
pub fn HomePage() -> Html {
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
                <UnpaginatedDetailTableRenderer mode={*table_mode} url={Rc::new(url)} {entry_count} sort=false />
            </Suspense>
        </>
    }
}
