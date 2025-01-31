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

use crate::components::tables::details::*;
use crate::contexts::WindowContext;

#[function_component]
pub fn UnverifiedPage() -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let entry_count = use_state_eq(|| None);

    let url = use_memo((), |()| {
        window_context.origin_join_segments(&["api", "titles", "unverified"])
    });

    let fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };

    html! {
        <>
            <h2>{"Unverified titles"}</h2>
            if let Some(count) = *entry_count {
                <span>
                    if count == 1 {
                        {"1 entry"}
                    } else {
                        {format!("{count} entries")}
                    }
                </span>
            }
            <Suspense {fallback}>
                <PaginatedDetailTableRenderer mode={DetailType::Title} {url} entry_count={entry_count.setter()} />
            </Suspense>
        </>
    }
}
