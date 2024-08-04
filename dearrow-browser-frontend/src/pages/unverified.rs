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

use crate::contexts::WindowContext;
use crate::components::detail_table::*;

#[function_component]
pub fn UnverifiedPage() -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let entry_count = use_state_eq(|| None);

    let url = window_context.origin.join("/api/titles/unverified").expect("Should be able to create an API url");

    let fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };
    
    html! {
        <>
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
                <PaginatedDetailTableRenderer mode={DetailType::Title} url={Rc::new(url)} {entry_count} />
            </Suspense>
        </>
    }
}
