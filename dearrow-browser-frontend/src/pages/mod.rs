/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2025 mini_bomba
*
*  This program is free software: you can redistribute it and/or modify
*  it under the terms of the GNU Affero General Public License as published by
*  the Free Software Foundation, either version 3 of the License, or
*  (at your option) any later version.
*
*  This program is distributed in the hope that it will be useful,
*  but WITHOUT ANY WARRANTY; without even the implied warranty of
*  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*  GNU Affero General Public License for more details.
*
*  You should have received a copy of the GNU Affero General Public License
*  along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;
use web_sys::window;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::components::header_footer::*;
use crate::components::modals::ModalRenderer;
use crate::components::tables::switch::Tabs;

mod broken;
mod channel;
mod home;
mod unverified;
mod user;
mod uuid;
mod video;

use broken::BrokenPage;
use channel::ChannelPage;
use home::HomePage;
use unverified::UnverifiedPage;
use user::UserPage;
use uuid::UUIDPage;
use video::VideoPage;

#[derive(Clone, Routable, PartialEq, IntoStaticStr)]
pub enum MainRoute {
    #[at("/")]
    Home,
    #[at("/unverified")]
    Unverified,
    #[at("/broken")]
    Broken,
    #[at("/video_id/:id")]
    Video { id: AttrValue },
    #[at("/channel/:id")]
    Channel { id: AttrValue },
    #[at("/user_id/:id")]
    User { id: AttrValue },
    #[at("/uuid/:id")]
    UUID { id: AttrValue },
    #[at("/wip")]
    NotImplemented,
    #[not_found]
    #[at("/404")]
    NotFound,
}

#[inline(always)]
fn is_default<T: Default + Eq>(n: &T) -> bool {
    *n == T::default()
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "T: Tabs")]
pub struct LocationState<T: Tabs>
{
    #[serde(default)]
    pub tab: T,
    #[serde(default, skip_serializing_if="is_default")]
    pub page: usize,
}

#[allow(clippy::needless_pass_by_value)]
pub fn render_main_route(route: MainRoute) -> Html {
    let document = window()
        .expect("window should exist")
        .document()
        .expect("document should exist");
    document.set_title(
        match &route {
            MainRoute::Home => "DeArrow Browser".to_string(),
            MainRoute::Unverified => "Unverified titles - DeArrow Browser".to_string(),
            MainRoute::Broken => "Broken entries - DeArrow Browser".to_string(),
            MainRoute::NotFound => "Page not found - DeArrow Browser".to_string(),
            MainRoute::NotImplemented => "Not implemented - DeArrow Browser".to_string(),
            MainRoute::Video { ref id } => format!("VideoID {id} - DeArrow Browser"),
            MainRoute::Channel { ref id } => format!("Channel {id} - DeArrow Browser"),
            MainRoute::User { ref id } => format!("UserID {id} - Dearrow Browser"),
            MainRoute::UUID { ref id } => format!("UUID {id} - Dearrow Browser"),
        }
        .as_str(),
    );
    let route_html = match route {
        MainRoute::Home => html! {<HomePage/>},
        MainRoute::Unverified => html! {<UnverifiedPage/>},
        MainRoute::Broken => html! {<BrokenPage/>},
        MainRoute::Video { ref id } => html! {<VideoPage videoid={id.clone()} />},
        MainRoute::Channel { ref id } => html! {<ChannelPage channel={id.clone()} />},
        MainRoute::User { ref id } => html! {<UserPage userid={id.clone()} />},
        MainRoute::UUID { ref id } => html! {<UUIDPage uuid={id.clone()} />},
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
