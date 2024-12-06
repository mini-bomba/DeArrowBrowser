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

use crate::components::searchbar::Searchbar;
use crate::components::tables::{details::*, switch::*};
use crate::contexts::{SettingsContext, StatusContext, WindowContext};
use crate::hooks::use_location_state;

#[function_component]
pub fn HomePage() -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let status_context: StatusContext = use_context().expect("StatusContext should be defined");
    let settings_context: SettingsContext =
        use_context().expect("SettingsContext should be available");
    let settings = settings_context.settings();
    let entries_per_page: usize = settings.entries_per_page.into();
    let state = use_location_state().get_state();

    let url_and_mode = use_memo(
        (
            state.detail_table_mode,
            entries_per_page,
            state.detail_table_page,
        ),
        |(dtm, entries_per_page, page)| {
            DetailType::try_from(*dtm).ok().map(|dtm| {
                let mut url = match dtm {
                    DetailType::Title => window_context.origin_join_segments(&["api", "titles"]),
                    DetailType::Thumbnail => {
                        window_context.origin_join_segments(&["api", "thumbnails"])
                    }
                };
                url.query_pairs_mut()
                    .append_pair("offset", &(page * entries_per_page).to_string())
                    .append_pair("count", &entries_per_page.to_string());
                (Rc::new(url), dtm)
            })
        },
    );

    let fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };

    let detail_count = status_context.and_then(|status_context| {
        url_and_mode.as_ref().as_ref().map(|(_, dtm)| match dtm {
            DetailType::Thumbnail => status_context.thumbnails,
            DetailType::Title => status_context.titles,
        })
    });
    let page_count =
        detail_count.map(|detail_count| detail_count.div_ceil(entries_per_page));

    html! {
        <>
            <div class="page-details">
                <Searchbar />
            </div>
            <TableModeSwitch entry_count={detail_count} types={ModeSubtype::Details} />
            if let Some((url, mode)) = url_and_mode.as_ref() {
                <Suspense {fallback}>
                    <UnpaginatedDetailTableRenderer mode={*mode} url={url.clone()} sort=false />
                </Suspense>
            } else {
                {fallback}
            }
            if let Some(page_count) = page_count {
                <PageSelect  {page_count} />
            }
        </>
    }
}
