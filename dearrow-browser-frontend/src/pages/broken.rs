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

use crate::components::detail_table::{DetailType, TableModeSwitch, PaginatedDetailTableRenderer};
use crate::contexts::WindowContext;
use crate::hooks::use_location_state;

#[function_component]
pub fn BrokenPage() -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let state = use_location_state().get_state();
    let entry_count = use_state_eq(|| None);

    let url = use_memo(state.detail_table_mode, |dtm| match dtm {
        DetailType::Title => window_context.origin_join_segments(&["api", "titles", "broken"]),
        DetailType::Thumbnail => window_context.origin_join_segments(&["api", "thumbnails", "broken"]),
    });

    let table_fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };
    
    html! {
        <>
            <h2>{"Broken database entries"}</h2>
            <TableModeSwitch entry_count={*entry_count} />
            <Suspense fallback={table_fallback}>
                <PaginatedDetailTableRenderer mode={state.detail_table_mode} {url} {entry_count} />
            </Suspense>
        </>
    }
}
