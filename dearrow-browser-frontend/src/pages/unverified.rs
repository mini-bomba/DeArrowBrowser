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

use dearrow_browser_api::unsync::ApiTitle;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use strum::{IntoStaticStr, VariantArray};
use yew::prelude::*;

use crate::components::tables::remote::{Endpoint, RemotePaginatedTable};
use crate::utils::ReqwestUrlExt;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, VariantArray, IntoStaticStr, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
enum UnverifiedPageTab {
    #[default]
    Titles,
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct UnverifiedTitles;

impl Endpoint for UnverifiedTitles {
    type Item = ApiTitle;
    type LoadProgress = ();

    fn create_url(&self, base_url: &Url) -> Url {
        base_url.join_segments(&["api", "titles", "unverified"]).expect("origin should be a valid base")
    }
}

#[function_component]
pub fn UnverifiedPage() -> Html {
    let item_count: UseStateHandle<Option<usize>> = use_state_eq(|| None);
    let callback = {
        let setter = item_count.setter();
        use_callback((), move |new, ()| setter.set(new))
    };

    html! {<>
        <h2>{"Unverified titles"}</h2>
        <span>
            if let Some(count) = *item_count {
                if count == 1 {
                    {"1 entry"}
                } else {
                    {format!("{count} entries")}
                }
            }
        </span>
        <RemotePaginatedTable<UnverifiedTitles, UnverifiedPageTab>
            endpoint={UnverifiedTitles}
            item_count_update={callback}
        />
    </>}
}
