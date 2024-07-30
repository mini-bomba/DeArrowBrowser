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

use dearrow_browser_api::unsync::StatusResponse;
use reqwest::Url;
use yew::prelude::*;
use yew_hooks::{use_local_storage, UseLocalStorageHandle};

pub use crate::components::modals::ModalRendererControls;
use crate::{settings::Settings, utils::ReqwestUrlExt};

#[derive(Clone, PartialEq)]
pub struct WindowContext {
    pub origin: Url,
    pub logo_url: Option<AttrValue>,
}

impl WindowContext {
    #[must_use]
    pub fn origin_join_segments<I>(&self, segments: I) -> Url
    where I: IntoIterator,
    I::Item: AsRef<str>,
    {
        self.origin.join_segments(segments).expect("WindowContext.origin should be a valid base")
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct UpdateClock(pub bool);

pub type StatusContext = Option<Rc<StatusResponse>>;

#[derive(Clone, PartialEq)]
pub struct SettingsContext {
    pub storage: UseLocalStorageHandle<Settings>,
    pub default: Rc<Settings>,
}

impl SettingsContext {
    pub fn settings(&self) -> &Settings {
        self.storage.as_ref().unwrap_or(&self.default)
    }

    pub fn update(&self, settings: Settings) {
        self.storage.set(settings);
    }
}

#[derive(Properties, PartialEq)]
pub struct SettingsProviderProps {
    pub children: Html
}

#[function_component]
pub fn SettingsProvider(props: &SettingsProviderProps) -> Html {
    let storage = use_local_storage("settings".into());
    let default = use_memo((), |()| Settings::default());
    let context = SettingsContext {
        storage,
        default,
    };

    html! {
        <ContextProvider<SettingsContext> {context}>
            {props.children.clone()}
        </ContextProvider<SettingsContext>>
    }
}
