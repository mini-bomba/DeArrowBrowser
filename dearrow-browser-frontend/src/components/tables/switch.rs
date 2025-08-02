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

use std::str::FromStr;

use strum::VariantArray;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::contexts::SettingsContext;
use crate::hooks::use_location_state;
use crate::pages::MainRoute;
use crate::pages::LocationState;

pub trait Tabs: VariantArray + Into<&'static str> + Copy + Sized + Default + Eq {}
impl<T> Tabs for T
where T: VariantArray + Into<&'static str> + Copy + Sized + Default + Eq {}

#[derive(Properties, PartialEq)]
pub struct TableModeSwitchProps {
    #[prop_or_default]
    pub entry_count: Option<usize>,
}

pub struct TableModeSwitch<T: Tabs> {
    current_mode: T,
    sticky_headers: bool,

    callbacks: Box<[Callback<MouseEvent>]>,

    _location_handle: LocationHandle,
    _settings_handle: ContextHandle<SettingsContext>,
}

pub enum TableModeSwitchMessage<T: Tabs> {
    UpdateMode(T),
    LocationUpdated(Location),
    SettingsUpdated(bool),
}

impl<T: Tabs> Component for TableModeSwitch<T> {
    type Properties = TableModeSwitchProps;
    type Message = TableModeSwitchMessage<T>;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();

        let location_state = match scope.location().unwrap().state::<LocationState<T>>() {
            Some(state) => *state,
            None => {
                let state = LocationState::default();
                scope
                    .navigator()
                    .unwrap()
                    .replace_with_state(&scope.route::<MainRoute>().unwrap(), state);
                state
            }
        };

        let (settings, settings_handle) = scope.context::<SettingsContext>(
            scope.callback(|s: SettingsContext| TableModeSwitchMessage::SettingsUpdated(s.settings().sticky_headers))
        ).expect("TableModeSwitch should be used inside of a SettingsProvider");

        Self {
            current_mode: location_state.detail_table_mode,
            sticky_headers: settings.settings().sticky_headers,

            callbacks: T::VARIANTS.iter()
                .copied()
                .map(|v| scope.callback(move |_| TableModeSwitchMessage::UpdateMode(v)))
                .collect(),

            _location_handle: scope
                .add_location_listener(scope.callback(TableModeSwitchMessage::LocationUpdated))
                .unwrap(),
            _settings_handle: settings_handle,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let classes = classes!("table-mode-switch", self.sticky_headers.then_some("sticky"));
        html! {
            <div class={classes}>
                {for T::VARIANTS.iter().copied().zip(self.callbacks.iter()).map(|(v, onclick)| html! {
                    <span class="table-mode button" {onclick} selected={self.current_mode == v}>{v.into()}</span>
                })}
                if let Some(count) = ctx.props().entry_count {
                    <span>
                        if count == 1 {
                            {"1 entry"}
                        } else {
                            {format!("{count} entries")}
                        }
                    </span>
                }
            </div>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Self::Message::LocationUpdated(location) => {
                let scope = ctx.link();
                let state = match location
                    .state::<LocationState<T>>()
                    .or_else(|| scope.location().unwrap().state())
                {
                    Some(state) => *state,
                    None => {
                        let state = LocationState::default();
                        scope
                            .navigator()
                            .unwrap()
                            .replace_with_state(&scope.route::<MainRoute>().unwrap(), state);
                        state
                    }
                };
                if self.current_mode == state.detail_table_mode {
                    false
                } else {
                    self.current_mode = state.detail_table_mode;
                    true
                }
            }
            Self::Message::UpdateMode(new_mode) if new_mode != self.current_mode => {
                self.current_mode = new_mode;
                let scope = ctx.link();
                scope
                    .navigator()
                    .unwrap()
                    .replace_with_state(
                        &scope.route::<MainRoute>().unwrap(),
                        LocationState {
                            detail_table_mode: new_mode,
                            detail_table_page: 0,
                        },
                    );
                true
            }
            Self::Message::SettingsUpdated(new_sticky) if new_sticky != self.sticky_headers => {
                self.sticky_headers = new_sticky;
                true
            }
            Self::Message::UpdateMode(..) | Self::Message::SettingsUpdated(..) => false,
        }
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct PageSelectProps {
    pub page_count: usize,
}

#[function_component]
pub fn PageSelect<T: Tabs>(props: &PageSelectProps) -> Html {
    let state_handle = use_location_state();
    let state = state_handle.get_state::<T>();

    let prev_page = {
        let state_handle = state_handle.clone();
        Callback::from(move |_| {
            let mut state = state;
            state.detail_table_page = state.detail_table_page.saturating_sub(1);
            state_handle.replace_state(state);
        })
    };
    let next_page = {
        let state_handle = state_handle.clone();
        let max_page = props.page_count - 1;
        Callback::from(move |_| {
            let mut state = state;
            state.detail_table_page = max_page.min(state.detail_table_page + 1);
            state_handle.replace_state(state);
        })
    };
    let input_changed = {
        let state_handle = state_handle.clone();
        let page_count = props.page_count;
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut state = state;
            match usize::from_str(&input.value()) {
                Err(_) => {}
                Ok(new_page) => {
                    state.detail_table_page = new_page.clamp(1, page_count) - 1;
                    state_handle.replace_state(state);
                }
            }
            input.set_value(&format!("{}", state.detail_table_page + 1));
        })
    };

    html! {
        <div class="page-select">
            <div class="button" onclick={prev_page}>{"prev"}</div>
            <div>
                {"page"}
                <input type="number" min=1 max={format!("{}", props.page_count)} ~value={format!("{}", state.detail_table_page+1)} onchange={input_changed} />
                {format!("/{}", props.page_count)}
            </div>
            <div class="button" onclick={next_page}>{"next"}</div>
        </div>
    }
}
