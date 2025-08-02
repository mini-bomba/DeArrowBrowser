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

use std::rc::Rc;

use cloneable_errors::ErrorContext;
use dearrow_browser_api::unsync::{ApiThumbnail, ApiTitle, ApiWarning, User};
use strum::{IntoStaticStr, VariantArray};
use yew::prelude::*;

use crate::components::icon::*;
use crate::components::tables::remote::{Endpoint, RemotePaginatedTable};
use crate::components::tables::switch::TableModeSwitch;
use crate::components::tables::thumbs::ThumbTableSettings;
use crate::components::tables::titles::TitleTableSettings;
use crate::components::tables::warnings::WarningTableSettings;
use crate::contexts::{StatusContext, WindowContext};
use crate::hooks::{use_async_suspension, use_location_state};
use crate::utils::{api_request, sbb_userid_link, ReqwestUrlExt};

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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, VariantArray, IntoStaticStr)]
enum UserPageTab {
    #[default]
    Titles,
    Thumbnails,
    #[strum(serialize = "Warnings received")]
    WarningsReceived,
    #[strum(serialize = "Warnings issued")]
    WarningsIssued,
}

#[derive(PartialEq, Eq, Clone)]
struct UserTitles {
    userid: AttrValue,
}
#[derive(PartialEq, Eq, Clone)]
struct UserThumbnails {
    userid: AttrValue,
}
#[derive(PartialEq, Eq, Clone)]
struct UserWarningsRcv {
    userid: AttrValue,
}
#[derive(PartialEq, Eq, Clone)]
struct UserWarningsIss {
    userid: AttrValue,
}

impl Endpoint for UserTitles {
    type Item = ApiTitle;
    type LoadProgress = ();

    fn create_url(&self, base_url: &reqwest::Url) -> reqwest::Url {
        base_url
            .join_segments(&["api", "titles", "user_id", &self.userid])
            .expect("base should be a valid base")
    }
}
impl Endpoint for UserThumbnails {
    type Item = ApiThumbnail;
    type LoadProgress = ();

    fn create_url(&self, base_url: &reqwest::Url) -> reqwest::Url {
        base_url
            .join_segments(&["api", "thumbnails", "user_id", &self.userid])
            .expect("base should be a valid base")
    }
}
impl Endpoint for UserWarningsRcv {
    type Item = ApiWarning;
    type LoadProgress = ();

    fn create_url(&self, base_url: &reqwest::Url) -> reqwest::Url {
        base_url
            .join_segments(&["api", "warnings", "user_id", &self.userid, "received"])
            .expect("base should be a valid base")
    }
}
impl Endpoint for UserWarningsIss {
    type Item = ApiWarning;
    type LoadProgress = ();

    fn create_url(&self, base_url: &reqwest::Url) -> reqwest::Url {
        base_url
            .join_segments(&["api", "warnings", "user_id", &self.userid, "issued"])
            .expect("base should be a valid base")
    }
}

#[derive(Properties, PartialEq)]
pub struct UserPageProps {
    pub userid: AttrValue,
}

#[function_component]
pub fn UserPage(props: &UserPageProps) -> Html {
    let state = use_location_state().get_state::<UserPageTab>();
    let entry_count = use_state_eq(|| None);
    let callback = {
        let setter = entry_count.setter();
        use_callback((), move |new, ()| setter.set(new))
    };

    let details_fallback = html! {
        <div><b>{"Loading..."}</b></div>
    };

    html! {
        <>
            <div class="page-details">
                <div class="info-table">
                    <Suspense fallback={details_fallback}><UserDetails userid={props.userid.clone()} /></Suspense>
                </div>
            </div>
            <TableModeSwitch<UserPageTab> entry_count={*entry_count} />
            {match state.detail_table_mode {
                UserPageTab::Titles => {
                    const SETTINGS: TitleTableSettings = TitleTableSettings {
                        hide_username: true,
                        hide_userid: true,
                        hide_videoid: false,
                    };
                    html! {
                        <RemotePaginatedTable<UserTitles, UserPageTab>
                            endpoint={UserTitles { userid: props.userid.clone() }}
                            settings={SETTINGS}
                            item_count_update={callback.clone()}
                        />
                    }
                }
                UserPageTab::Thumbnails => {
                    const SETTINGS: ThumbTableSettings = ThumbTableSettings {
                        hide_username: true,
                        hide_userid: true,
                        hide_videoid: false,
                    };
                    html! {
                        <RemotePaginatedTable<UserThumbnails, UserPageTab>
                            endpoint={UserThumbnails { userid: props.userid.clone() }}
                            settings={SETTINGS}
                            item_count_update={callback.clone()}
                        />
                    }
                }
                UserPageTab::WarningsIssued => {
                    const SETTINGS: WarningTableSettings = WarningTableSettings {
                        hide_issuer: true,
                        hide_receiver: false,
                    };
                    html! {
                        <RemotePaginatedTable<UserWarningsIss, UserPageTab>
                            endpoint={UserWarningsIss { userid: props.userid.clone() }}
                            settings={SETTINGS}
                            item_count_update={callback.clone()}
                        />
                    }
                }
                UserPageTab::WarningsReceived => {
                    const SETTINGS: WarningTableSettings = WarningTableSettings {
                        hide_issuer: false,
                        hide_receiver: true,
                    };
                    html! {
                        <RemotePaginatedTable<UserWarningsRcv, UserPageTab>
                            endpoint={UserWarningsRcv { userid: props.userid.clone() }}
                            settings={SETTINGS}
                            item_count_update={callback.clone()}
                        />
                    }
                }
            }}
        </>
    }
}
