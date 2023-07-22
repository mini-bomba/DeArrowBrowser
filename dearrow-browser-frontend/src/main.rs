use yew::prelude::*;
use web_sys::HtmlInputElement;

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

#[function_component]
fn App() -> Html {
    let url = use_state(|| "".to_string());
    let onkeydown = {
        let url = url.clone();
        move |e: KeyboardEvent| {
            if e.key() != "Enter" {
                return;
            }
            let input: HtmlInputElement = e.target_unchecked_into();
            url.set(input.value());
        }
    };
    
    let fallback = html! {<div>{"Loading..."}</div>};
    html! {
        <>
            <div>
                <input placeholder="Enter an URL here" {onkeydown} />
            </div>
            <Suspense {fallback}>
                <RequestRenderer url={url.to_string()} />
            </Suspense>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
