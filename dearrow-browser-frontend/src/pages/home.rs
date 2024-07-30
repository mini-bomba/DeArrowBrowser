/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2024 mini_bomba
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
use std::rc::Rc;

use yew::prelude::*;

use crate::{components::{detail_table::*, searchbar::Searchbar}, contexts::{StatusContext, WindowContext, SettingsContext}, hooks::use_location_state};

#[function_component]
pub fn HomePage() -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let status_context: StatusContext = use_context().expect("StatusContext should be defined");
    let settings_context: SettingsContext = use_context().expect("SettingsContext should be available");
    let settings = settings_context.settings();
    let entries_per_page: usize = settings.entries_per_page.into();
    let state = use_location_state().get_state();

    let mut url = match state.detail_table_mode {
        DetailType::Title => window_context.origin.join("/api/titles"),
        DetailType::Thumbnail => window_context.origin.join("/api/thumbnails"),
    }.expect("Should be able to create an API url");

    url.query_pairs_mut()
        .append_pair("offset", &format!("{}", state.detail_table_page*entries_per_page))
        .append_pair("count", &entries_per_page.to_string());

    let fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };

    let detail_count = status_context.map(|status_context| 
        match state.detail_table_mode {
            DetailType::Thumbnail => status_context.thumbnails,
            DetailType::Title     => status_context.titles,
        }
    );
    let page_count = detail_count.map(|detail_count| (detail_count+(entries_per_page-1))/entries_per_page);
    
    html! {
        <>
            <div class="page-details">
                <Searchbar />
            </div>
            <TableModeSwitch entry_count={detail_count} />
            <Suspense {fallback}>
                <UnpaginatedDetailTableRenderer mode={state.detail_table_mode} url={Rc::new(url)} sort=false />
            </Suspense>
            if let Some(page_count) = page_count {
                <PageSelect  {page_count} />
            }
        </>
    }
}
