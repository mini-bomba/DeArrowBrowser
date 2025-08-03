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

use dearrow_browser_api::unsync::{ApiCasualTitle, ApiThumbnail, ApiTitle};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use strum::{IntoStaticStr, VariantArray};
use yew::prelude::*;

use crate::components::searchbar::Searchbar;
use crate::components::tables::remote::{Endpoint, RemoteUnpaginatedTable};
use crate::components::tables::switch::*;
use crate::contexts::{SettingsContext, StatusContext};
use crate::hooks::use_location_state;
use crate::utils::ReqwestUrlExt;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, VariantArray, IntoStaticStr, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
enum HomePageTab {
    #[default]
    Titles,
    Thumbnails,
    #[strum(serialize="Casual titles")]
    CasualTitles,
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct HomePageTitles {
    offset: usize,
    count: usize,
}
#[derive(PartialEq, Eq, Clone, Copy)]
struct HomePageThumbnails {
    offset: usize,
    count: usize,
}
#[derive(PartialEq, Eq, Clone, Copy)]
struct HomePageCasualTitles {
    offset: usize,
    count: usize,
}

impl Endpoint for HomePageTitles {
    type Item = ApiTitle;
    type LoadProgress = ();

    fn create_url(&self, base_url: &Url) -> Url {
        let mut url = base_url
            .join_segments(&["api", "titles"])
            .expect("origin should be a valid base");
        url.query_pairs_mut()
            .append_pair("offset", &self.offset.to_string())
            .append_pair("count", &self.count.to_string());
        url
    }
}
impl Endpoint for HomePageThumbnails {
    type Item = ApiThumbnail;
    type LoadProgress = ();

    fn create_url(&self, base_url: &Url) -> Url {
        let mut url = base_url
            .join_segments(&["api", "thumbnails"])
            .expect("origin should be a valid base");
        url.query_pairs_mut()
            .append_pair("offset", &self.offset.to_string())
            .append_pair("count", &self.count.to_string());
        url
    }
}
impl Endpoint for HomePageCasualTitles {
    type Item = ApiCasualTitle;
    type LoadProgress = ();

    fn create_url(&self, base_url: &Url) -> Url {
        let mut url = base_url
            .join_segments(&["api", "casual_titles"])
            .expect("origin should be a valid base");
        url.query_pairs_mut()
            .append_pair("offset", &self.offset.to_string())
            .append_pair("count", &self.count.to_string());
        url
    }
}

#[function_component]
pub fn HomePage() -> Html {
    let status: StatusContext = use_context().expect("Status should be available");
    let settings_ctx: SettingsContext = use_context().expect("Settings should be available");
    let settings = settings_ctx.settings();
    let entries_per_page = settings.entries_per_page.get();
    let state = use_location_state().get_state::<HomePageTab>();

    match state.tab {
        HomePageTab::Titles => html! {<>
            <div class="page-details">
                <Searchbar />
            </div>
            <TableModeSwitch<HomePageTab> entry_count={status.as_ref().and_then(|s| s.titles)} />
            <RemoteUnpaginatedTable<HomePageTitles> endpoint={HomePageTitles {
                offset: state.page * entries_per_page,
                count: entries_per_page,
            }} />
            if let Some(page_count) = status.as_ref().and_then(|s| s.titles.map(|c| c.div_ceil(entries_per_page))) {
                <PageSelect<HomePageTab> {page_count} />
            }
        </>},
        HomePageTab::Thumbnails => html! {<>
            <div class="page-details">
                <Searchbar />
            </div>
            <TableModeSwitch<HomePageTab> entry_count={status.as_ref().and_then(|s| s.thumbnails)} />
            <RemoteUnpaginatedTable<HomePageThumbnails> endpoint={HomePageThumbnails {
                offset: state.page * entries_per_page,
                count: entries_per_page,
            }} />
            if let Some(page_count) = status.as_ref().and_then(|s| s.thumbnails.map(|c| c.div_ceil(entries_per_page))) {
                <PageSelect<HomePageTab> {page_count} />
            }
        </>},
        HomePageTab::CasualTitles => html! {<>
            <div class="page-details">
                <Searchbar />
            </div>
            <TableModeSwitch<HomePageTab> entry_count={status.as_ref().and_then(|s| s.casual_titles)} />
            <RemoteUnpaginatedTable<HomePageCasualTitles> endpoint={HomePageCasualTitles {
                offset: state.page * entries_per_page,
                count: entries_per_page,
            }} />
            if let Some(page_count) = status.as_ref().and_then(|s| s.casual_titles.map(|c| c.div_ceil(entries_per_page))) {
                <PageSelect<HomePageTab> {page_count} />
            }
        </>},
    }
}
