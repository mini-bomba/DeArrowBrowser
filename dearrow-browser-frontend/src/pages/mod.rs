use strum::IntoStaticStr;
use web_sys::window;
use yew_router::prelude::*;
use yew::prelude::*;

use crate::components::{header_footer::*, modals::ModalRenderer, detail_table::DetailType};

mod home;
mod unverified;
mod user;
mod video;

use home::HomePage;
use unverified::UnverifiedPage;
use user::UserPage;
use video::VideoPage;

#[derive(Clone, Routable, PartialEq, IntoStaticStr)]
pub enum MainRoute {
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

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct LocationState {
    pub detail_table_mode: DetailType,
    pub detail_table_page: usize,
}

#[allow(clippy::needless_pass_by_value)]
pub fn render_main_route(route: MainRoute) -> Html {
    let document = window().expect("window should exist")
        .document().expect("document should exist");
    document.set_title(match &route {
        MainRoute::Home => "DeArrow Browser".to_string(),
        MainRoute::Unverified => "Unverified titles - DeArrow Browser".to_string(),
        MainRoute::NotFound => "Page not found - DeArrow Browser".to_string(),
        MainRoute::NotImplemented => "Not implemented - DeArrow Browser".to_string(),
        MainRoute::Video { ref id } => format!("VideoID {id} - DeArrow Browser"),
        MainRoute::User { ref id } => format!("UserID {id} - Dearrow Browser"),
    }.as_str());
    let route_html = match route {
        MainRoute::Home => html! {<HomePage/>},
        MainRoute::Unverified => html! {<UnverifiedPage/>},
        MainRoute::Video { ref id } => html! {<VideoPage videoid={id.clone()} />},
        MainRoute::User { ref id } => html! {<UserPage userid={id.clone()} />},
        MainRoute::NotFound => html! {
            <>
                <h2>{"404 - Not found"}</h2>
                <h3>{"Looks like you've entered an invalid URL"}</h3>
                <Link<MainRoute> to={MainRoute::Home}>{"Return to home page"}</Link<MainRoute>>
            </>
        },
        MainRoute::NotImplemented => html! {
            <>
                <h2>{"Not implemented"}</h2>
                <h3>{"This feature is not implemented yet"}</h3>
                <Link<MainRoute> to={MainRoute::Home}>{"Return to home page"}</Link<MainRoute>>
            </>
        },
    };
    let route_name: &'static str = (&route).into();
    html! {
        <ModalRenderer>
            <Header />
            <div id="content" data-route={route_name}>
                {route_html}
            </div>
            <Footer />
        </ModalRenderer>
    }
}
