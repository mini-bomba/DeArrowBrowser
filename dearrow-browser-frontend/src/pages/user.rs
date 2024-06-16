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

use dearrow_browser_api::unsync::User;
use yew::prelude::*;

use crate::{contexts::{WindowContext, StatusContext}, hooks::{use_async_suspension, use_location_state}, components::detail_table::*};

#[derive(Properties, PartialEq)]
struct UserDetailsProps {
    userid: AttrValue,
}

#[function_component]
fn UserDetails(props: &UserDetailsProps) -> HtmlResult {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let status: StatusContext = use_context().expect("StatusResponse should be defined");
    let url = window_context.origin_join_segments(&["api","users","user_id", &props.userid]);
    let result: Rc<Result<User, anyhow::Error>> = use_async_suspension(|(url, _)| async move {
        Ok(reqwest::get((url).clone()).await?.json().await?)
    }, (url, status.map(|s| s.last_updated)))?;

    Ok(match *result {
        Ok(ref user) => html! {
            <>
                <div>{format!("UserID: {}", props.userid.clone())}
                if user.vip {
                   <span title="This user is a VIP">{"👑"}</span> 
                }
                </div> 
                <div>
                if let Some(username) = &user.username {
                    {format!("Username: {username}")}
                } else {
                    {"Username: "}<em>{"No username set"}</em>
                }
                if user.username_locked {
                    <span title="This user's username is locked">{"🔒"}</span>
                }
                </div>
                <div>{format!("Titles: {}", user.title_count)}</div>
                <div>{format!("Thumbnails: {}", user.thumbnail_count)}</div>
            </>
        },
        Err(ref e) => html! {
            <div>{"Failed to fetch user data"}<br/><pre>{format!("{e:?}")}</pre></div>
        }
    })
}

#[derive(Properties, PartialEq)]
pub struct UserPageProps {
    pub userid: AttrValue,
}

#[function_component]
pub fn UserPage(props: &UserPageProps) -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let state = use_location_state().get_state();
    // let table_mode = use_state_eq(|| DetailType::Title);
    let entry_count = use_state_eq(|| None);

    let url = match state.detail_table_mode {
        DetailType::Title => window_context.origin_join_segments(&["api", "titles", "user_id", &props.userid]),
        DetailType::Thumbnail => window_context.origin_join_segments(&["api", "thumbnails", "user_id", &props.userid]),
    };

    let details_fallback = html! {
        <div><b>{"Loading..."}</b></div>
    };
    let table_fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };
    
    html! {
        <>
            <div id="page-details">
                <div id="details-table">
                    <Suspense fallback={details_fallback}><UserDetails userid={props.userid.clone()} /></Suspense>
                </div>
            </div>
            <TableModeSwitch entry_count={*entry_count} />
            <Suspense fallback={table_fallback}>
                <PaginatedDetailTableRenderer mode={state.detail_table_mode} url={Rc::new(url)} {entry_count} hide_userid=true hide_username=true />
            </Suspense>
        </>
    }
}
