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

use dearrow_browser_api::unsync::{ApiThumbnail, ApiTitle};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use strum::{IntoStaticStr, VariantArray};
use yew::prelude::*;

use crate::components::tables::remote::{Endpoint, RemotePaginatedTable};
use crate::components::tables::switch::TableModeSwitch;
use crate::hooks::use_location_state;
use crate::utils::ReqwestUrlExt;


#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, VariantArray, IntoStaticStr, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
enum BrokenPageTab {
    #[default]
    Titles,
    Thumbnails,
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct BrokenTitles;
#[derive(PartialEq, Eq, Clone, Copy)]
struct BrokenThumbnails;

impl Endpoint for BrokenTitles {
    type Item = ApiTitle;
    type LoadProgress = ();

    fn create_url(&self, base_url: &Url) -> Url {
        base_url.join_segments(&["api", "titles", "broken"]).expect("origin should be a valid base")
    }
}
impl Endpoint for BrokenThumbnails {
    type Item = ApiThumbnail;
    type LoadProgress = ();

    fn create_url(&self, base_url: &Url) -> Url {
        base_url.join_segments(&["api", "thumbnails", "broken"]).expect("origin should be a valid base")
    }
}

#[function_component]
pub fn BrokenPage() -> Html {
    let item_count: UseStateHandle<Option<usize>> = use_state_eq(|| None);
    let state = use_location_state().get_state::<BrokenPageTab>();
    let callback = {
        let setter = item_count.setter();
        use_callback((), move |new, ()| setter.set(new))
    };

    html! {<>
        <h2>{"Broken database entries"}</h2>
        <TableModeSwitch<BrokenPageTab> entry_count={*item_count} />
        if let BrokenPageTab::Titles = state.tab {
            <RemotePaginatedTable<BrokenTitles, BrokenPageTab>
                endpoint={BrokenTitles}
                item_count_update={callback}
            />
        } else {
            <RemotePaginatedTable<BrokenThumbnails, BrokenPageTab>
                endpoint={BrokenThumbnails}
                item_count_update={callback}
            />
        }
    </>}
}
