/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2026 mini_bomba
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

use gloo_console::error;
use wasm_bindgen::prelude::Closure;
use web_sys::{window, StorageEvent};
use yew::{html::ChildrenProps, prelude::*};

use super::theme::ThemeRenderer;
use super::user::UserContextProvider;
use crate::settings::Settings;
use crate::utils_common::EventListener;

#[derive(Clone, PartialEq)]
pub struct SettingsContext {
    storage: Option<Rc<Settings>>,
    pub default: Rc<Settings>,
    update_callback: Callback<Settings>,
}

impl SettingsContext {
    pub fn settings(&self) -> &Settings {
        self.storage.as_ref().unwrap_or(&self.default)
    }

    pub fn update(&self, settings: Settings) {
        self.update_callback.emit(settings);
    }
}

pub struct SettingsProvider {
    context: SettingsContext,

    _event_listener: EventListener<dyn Fn(StorageEvent)>,
}

pub enum SettingsProviderMessage {
    LocalUpdate(Settings),
    RemoteUpdate(StorageEvent),
}

macro_rules! handle_jserr {
    ($ctx:literal) => {
        |err| {
            error!($ctx, err);
            Default::default()
        }
    };
}

impl SettingsProvider {
    fn load_from_storage(&mut self) -> Option<()> {
        let setting_json = window()?
            .local_storage()
            .unwrap_or_else(handle_jserr!("Failed to load settings from local storage: Error while accessing window.localStorage:"))?
            .get_item("settings")
            .unwrap_or_else(handle_jserr!("Failed to load settings from local storage: Error while retrieving the 'settings' key:"))?;
        match serde_json::from_str(&setting_json) {
            Ok(s) => self.context.storage = Some(Rc::new(s)),
            Err(err) => error!(format!("Failed to load setting from local storage: JSON deserialization error: {err:?}")),
        }
        None
    }

    fn save_to_storage(&self) -> Option<()> {
        let setting_json = if let Some(ref storage) = self.context.storage {
            match serde_json::to_string(storage) {
                Ok(s) => s,
                Err(err) => {
                    error!(format!("Failed to save settings to local storage: JSON serialization error: {err:?}"));
                    return None;
                }
            }
        } else {
            // nothing to save ???
            return None;
        };
        window()?
            .local_storage()
            .unwrap_or_else(handle_jserr!("Failed to save settings to local storage: Error while accessing window.localStorage:"))?
            .set_item("settings", &setting_json)
            .unwrap_or_else(handle_jserr!("Failed to save settings to local storage: Error while setting the 'settings' key:"));
        None
    }
}

impl Component for SettingsProvider {
    type Message = SettingsProviderMessage;
    type Properties = ChildrenProps;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();
        let the_window = window().expect("window should exist");
        let remote_callback = scope.callback(SettingsProviderMessage::RemoteUpdate);
        let mut this = Self {
            context: SettingsContext {
                storage: None,
                default: Rc::default(),
                update_callback: scope.callback(SettingsProviderMessage::LocalUpdate),
            },
            _event_listener: EventListener::new(
                &the_window,
                "storage",
                Closure::own(move |e| remote_callback.emit(e)),
            )
            .expect("failed to listen for storage events"),
        };
        this.load_from_storage();
        this
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <ContextProvider<SettingsContext> context={self.context.clone()}>
                <ThemeRenderer />
                <UserContextProvider>
                    {ctx.props().children.clone()}
                </UserContextProvider>
            </ContextProvider<SettingsContext>>
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Self::Message::LocalUpdate(new) if self.context.storage.as_ref().is_some_and(|old| **old != new) => {
                self.context.storage = Some(Rc::new(new));
                self.save_to_storage();
                true
            }
            // no changes
            Self::Message::LocalUpdate(..) => false,
            Self::Message::RemoteUpdate(event) => {
                if event.key().is_none_or(|k| k != "settings") {
                    return false;
                }
                let Some(new_value) = event.new_value() else {
                    return false;
                };
                let new_settings = match serde_json::from_str::<Settings>(&new_value) {
                    Ok(v) => v,
                    Err(err) => {
                        error!(format!("Failed to deserialize new settings received from storage event: {err:?}"));
                        return false;
                    }
                };
                if self.context.storage.as_ref().is_some_and(|old| **old == new_settings) {
                    return false;
                }
                self.context.storage = Some(Rc::new(new_settings));
                true
            }
        }
    }
}
