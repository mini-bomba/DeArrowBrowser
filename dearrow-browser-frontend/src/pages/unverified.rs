use std::rc::Rc;

use yew::prelude::*;

use crate::{contexts::WindowContext, components::detail_table::*};

#[function_component]
pub fn UnverifiedPage() -> Html {
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
