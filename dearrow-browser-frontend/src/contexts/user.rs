/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024 mini_bomba
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
use dearrow_browser_api::unsync::User;
use gloo_console::error;
use yew::html::ChildrenProps;
use yew::prelude::*;
use yew::ContextHandle;

use crate::contexts::{
    misc::{StatusContext, WindowContext},
    settings::SettingsContext,
};
use crate::utils_app::sponsorblock_hash;
use crate::utils_common::api_request;

pub type UserContext = Option<UserContextData>;
#[derive(Clone, PartialEq)]
pub struct UserContextData {
    pub user_id: Rc<str>,
    pub data: Option<Result<Rc<User>, ErrorContext>>,
}

pub(super) struct UserContextProvider {
    private_user_id: Option<Rc<str>>,
    user_data: UserContext,
    last_update: Option<i64>,
    _settings_context_handle: ContextHandle<SettingsContext>,
    _status_context_handle: ContextHandle<StatusContext>,
}

pub(super) enum UserContextProviderMessage {
    UserIdUpdate(Option<Rc<str>>),
    UserLookupFinished{ 
        public_user_id: Rc<str>,
        result: Result<Rc<User>, ErrorContext>,
    },
    StatusUpdate(Option<i64>),
}

async fn fetch_user_data(window_context: Rc<WindowContext>, public_id: Rc<str>) -> UserContextProviderMessage {
    let url = window_context.origin_join_segments(&["api", "users", "user_id", &public_id]);
    UserContextProviderMessage::UserLookupFinished { 
        public_user_id: public_id,
        result: api_request::<_, User>(url).await
            .map(Rc::new)
            .inspect_err(|err| error!(format!("Failed to fetch current user data: {err:?}")))
    }
}

impl Component for UserContextProvider {
    type Message = UserContextProviderMessage;
    type Properties = ChildrenProps;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();
        let (settings, settings_handle) = scope.context(scope.callback(|s: SettingsContext| UserContextProviderMessage::UserIdUpdate(s.settings().private_user_id.clone()))).unwrap();
        let (status, status_handle) = scope.context(scope.callback(|s: StatusContext| UserContextProviderMessage::StatusUpdate(s.and_then(|s| s.last_updated)))).unwrap();
        let window_context = scope.context(Callback::noop()).unwrap().0; // we don't care about updates
        let private_user_id = settings.settings().private_user_id.clone();
        UserContextProvider {
            user_data: private_user_id.as_ref().map(|private_user_id| {
                let user_id: Rc<str> = sponsorblock_hash(private_user_id.as_bytes(), 5000).into();
                scope.send_future(fetch_user_data(window_context, user_id.clone()));
                UserContextData {
                    user_id,
                    data: None,
                }
            }),
            private_user_id,
            last_update: status.and_then(|s| s.last_updated),
            _settings_context_handle: settings_handle,
            _status_context_handle: status_handle,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <ContextProvider<UserContext> context={self.user_data.clone()}>
                {ctx.props().children.clone()}
            </ContextProvider<UserContext>>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        #[allow(clippy::match_same_arms)]
        match msg {
            UserContextProviderMessage::UserIdUpdate(private_user_id) if self.private_user_id != private_user_id => {
                self.private_user_id = private_user_id;
                self.user_data = self.private_user_id.as_ref().map(|private_user_id| {
                    let user_id: Rc<str> = sponsorblock_hash(private_user_id.as_bytes(), 5000).into();
                    let scope = ctx.link();
                    let window_context = scope.context(Callback::noop()).unwrap().0; // we don't care about updates
                    scope.send_future(fetch_user_data(window_context, user_id.clone()));
                    UserContextData {
                        user_id,
                        data: None,
                    }
                });
                true
            },
            // userid changed in the meantime
            UserContextProviderMessage::UserIdUpdate(..) => false,
            UserContextProviderMessage::UserLookupFinished { public_user_id, result } => {
                match self.user_data {
                    None => false,
                    Some(ref mut user_data) => {
                        if user_data.user_id != public_user_id { return false; }
                        user_data.data = Some(result);
                        true
                    }
                }
            },
            UserContextProviderMessage::StatusUpdate(Some(last_updated)) if self.last_update.is_none_or(|v| v != last_updated) => {
                self.last_update.replace(last_updated).is_some()
            },
            // Some(last_updated == self.last_update) - we don't care
            // None - status should never get initialized
            UserContextProviderMessage::StatusUpdate(..) => false,
        }
    }
}
