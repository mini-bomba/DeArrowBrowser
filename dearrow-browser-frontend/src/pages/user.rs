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
use cloneable_errors::ErrorContext;
use yew::prelude::*;

use crate::components::icon::*;
use crate::components::tables::details::*;
use crate::components::tables::switch::{ModeSubtype, TableMode, TableModeSwitch};
use crate::components::tables::warnings::PaginatedWarningsTable;
use crate::contexts::{StatusContext, WindowContext};
use crate::hooks::{use_async_suspension, use_location_state};
use crate::utils::{api_request, sbb_userid_link};

#[derive(Properties, PartialEq)]
struct UserDetailsProps {
    userid: AttrValue,
}

#[function_component]
fn UserDetails(props: &UserDetailsProps) -> HtmlResult {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let status: StatusContext = use_context().expect("StatusResponse should be defined");
    let url = window_context.origin_join_segments(&["api", "users", "user_id", &props.userid]);
    let result: Rc<Result<User, ErrorContext>> = use_async_suspension(
        |(url, _)| async move { api_request(url.clone()).await },
        (url, status.map(|s| s.last_updated)),
    )?;
    let sbb_url: Rc<AttrValue> = use_memo(props.userid.clone(), |uid| {
        AttrValue::Rc(sbb_userid_link(uid).as_str().into())
    });

    Ok(match *result {
        Ok(ref user) => html! {
            <>
                <div>{format!("UserID: {}", props.userid.clone())}
                if user.vip {
                    <Icon r#type={IconType::VIP} tooltip="This user is a VIP" />
                }
                if user.active_warning_count > 0 {
                    <Icon r#type={IconType::Warning} tooltip="This user has an active warning" />
                } else if user.warning_count > 0 {
                    <Icon r#type={IconType::WarningInactive} tooltip="This user was previously warned" />
                }
                </div>
                <div>
                if let Some(username) = &user.username {
                    {format!("Username: {username}")}
                } else {
                    {"Username: "}<em>{"No username set"}</em>
                }
                if user.username_locked {
                    <Icon r#type={IconType::Locked} tooltip="This user's username is locked" />
                }
                </div>
                <div>{format!("Titles: {}", user.title_count)}</div>
                <div>{format!("Thumbnails: {}", user.thumbnail_count)}</div>
                <div><a href={&*sbb_url}>{"View on SB Browser"}</a></div>
            </>
        },
        Err(ref e) => html! {
            <>
                <div>{"Failed to fetch user data"}<br/><pre>{format!("{e:?}")}</pre></div>
                <div><a href={&*sbb_url}>{"View on SB Browser"}</a></div>
            </>
        },
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
    let entry_count = use_state_eq(|| None);

    let details_fallback = html! {
        <div><b>{"Loading..."}</b></div>
    };
    let table_fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };

    let table_html = use_memo(
        (state.detail_table_mode, props.userid.clone()),
        |(dtm, userid)| match dtm {
            TableMode::Titles => {
                let url = Rc::new(
                    window_context.origin_join_segments(&["api", "titles", "user_id", userid]),
                );
                html! {
                    <Suspense fallback={table_fallback.clone()}>
                        <PaginatedDetailTableRenderer mode={DetailType::Title} {url} entry_count={entry_count.setter()} hide_userid=true hide_username=true />
                    </Suspense>
                }
            }
            TableMode::Thumbnails => {
                let url = Rc::new(window_context.origin_join_segments(&[
                    "api",
                    "thumbnails",
                    "user_id",
                    userid,
                ]));
                html! {
                    <Suspense fallback={table_fallback.clone()}>
                        <PaginatedDetailTableRenderer mode={DetailType::Thumbnail} {url} entry_count={entry_count.setter()} hide_userid=true hide_username=true />
                    </Suspense>
                }
            }
            TableMode::WarningsIssued => {
                let url = Rc::new(
                    window_context
                        .origin_join_segments(&["api", "warnings", "user_id", userid, "issued"]),
                );
                html! {
                    <PaginatedWarningsTable {url} entry_count={entry_count.setter()} hide_issuer=true />
                }
            }
            TableMode::WarningsReceived => {
                let url = Rc::new(
                    window_context
                        .origin_join_segments(&["api", "warnings", "user_id", userid, "received"]),
                );
                html! {
                    <PaginatedWarningsTable {url} entry_count={entry_count.setter()} hide_receiver=true />
                }
            }
        },
    );

    html! {
        <>
            <div class="page-details">
                <div class="info-table">
                    <Suspense fallback={details_fallback}><UserDetails userid={props.userid.clone()} /></Suspense>
                </div>
            </div>
            <TableModeSwitch entry_count={*entry_count} types={ModeSubtype::Details | ModeSubtype::Warnings} />
            {(*table_html).clone()}
        </>
    }
}
