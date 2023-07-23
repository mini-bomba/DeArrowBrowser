use yew::prelude::*;
use web_sys::window;

mod hooks;
use hooks::use_async_with_deps;

#[derive(Properties, PartialEq)]
pub struct RequestRendererProps {
    url: AttrValue,
}

#[function_component]
fn RequestRenderer(props: &RequestRendererProps) -> HtmlResult {
    let request = use_async_with_deps(|url| async move {
        reqwest::get(url.as_str()).await?.text().await
    }, props.url.clone())?;
    match *request {
        Err(_) => Ok(html! {
            <b>{format!("Failed to fetch {}", props.url)}</b>
        }),
        Ok(ref text) => Ok(html! {
            <pre>{text}</pre>
        }),
    }
}

macro_rules! search_block {
    ($id:expr, $name:expr) => {
        html! {
            <div>
                <label for={$id} >{concat!("Search by ", $name)}</label>
                <input id={$id} placeholder={$name} value="" />
            </div>
        }
    };
}

#[function_component]
fn App() -> Html {
    let logo_url = use_memo(|_| {
        window()
            .and_then(|w| w.document())
            .and_then(|d| d.query_selector("link[rel=icon]").ok().flatten())
            .and_then(|el| el.get_attribute("href"))
            .map(AttrValue::from)
    }, ());
    
    let logo = match *logo_url {
        None => html! {},
        Some(ref url) => html! { <img src={url} /> },
    };
    html! {
        <>
            <div class="header">
                {logo}
                <div>
                    <h2>{"DeArrow Browser"}</h2>
                </div>
            </div>
            <div class="searchbar">
                {search_block!("uuid_search", "UUID")}
                {search_block!("vid_search", "Video ID")}
                {search_block!("uid_search", "User ID")}
            </div>
            <div class="footer">
                <span>{"Last update: ..."}</span>
                <span>
                    {"DeArrow Browser © mini_bomba 2023. Uses DeArrow data licensed under "}
                    <a href="https://creativecommons.org/licenses/by-nc-sa/4.0/">{"CC BY-NC-SA 4.0"}</a>
                    {" from "}
                    <a href="https://dearrow.ajay.app/">{"https://dearrow.ajay.app/"}</a>
                    {"."}
                </span>
            </div>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
