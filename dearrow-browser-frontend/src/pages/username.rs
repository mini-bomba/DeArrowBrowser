/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2025 mini_bomba
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

use dearrow_browser_api::unsync::User;
use serde::{Deserialize, Serialize};
use strum::{IntoStaticStr, VariantArray};
use yew::{
    function_component, html, use_callback, use_state_eq, AttrValue, Html, Properties,
    UseStateHandle,
};

use crate::{
    components::tables::remote::{Endpoint, RemotePaginatedTable},
    utils_common::ReqwestUrlExt,
};

#[derive(
    Debug, PartialEq, Eq, Clone, Copy, Default, VariantArray, IntoStaticStr, Serialize, Deserialize,
)]
#[serde(rename_all="snake_case")]
enum UsernamePageTab {
    #[default]
    Users,
}

#[derive(PartialEq, Eq)]
struct UsernamePageEndpoint {
    username: AttrValue,
}

impl Endpoint for UsernamePageEndpoint {
    type Item = User;
    type LoadProgress = ();

    fn create_url(&self, base_url: &reqwest::Url) -> reqwest::Url {
        base_url
            .join_segments(&["api", "users", "username", &self.username])
            .expect("base_url should be a valid base")
    }
}

#[derive(Properties, PartialEq)]
pub struct UsernameProps {
    pub username: AttrValue,
}

#[function_component]
pub fn UsernamePage(props: &UsernameProps) -> Html {
    let item_count: UseStateHandle<Option<usize>> = use_state_eq(|| None);
    let callback = {
        let setter = item_count.setter();
        use_callback((), move |new, ()| setter.set(new))
    };

    html! {<>
        <h2>{"Users with username "}{&props.username}</h2>
        if let Some(count) = *item_count {
            <div>{format!("{count} results")}</div>
        }
        <RemotePaginatedTable<UsernamePageEndpoint, UsernamePageTab> 
            endpoint={UsernamePageEndpoint {
                username: props.username.clone(),
            }}
            item_count_update={callback}
        />
    </>}
}
