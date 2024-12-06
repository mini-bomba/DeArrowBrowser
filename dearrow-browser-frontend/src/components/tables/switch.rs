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

use std::ops::BitOr;
use std::str::FromStr;

use enumflags2::{bitflags, BitFlags};
use html::IntoPropValue;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::hooks::use_location_state;
use crate::pages::LocationState;
use crate::pages::MainRoute;

use super::details::DetailType;

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum TableMode {
    #[default]
    Titles,
    Thumbnails,
    WarningsReceived,
    WarningsIssued,
}

impl TryFrom<TableMode> for DetailType {
    type Error = ();

    fn try_from(value: TableMode) -> Result<Self, Self::Error> {
        match value {
            TableMode::Titles => Ok(DetailType::Title),
            TableMode::Thumbnails => Ok(DetailType::Thumbnail),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ModeSubtype {
    Details,
    Warnings,
}

#[bitflags]
#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy)]
enum InternalModeSubtype {
    Details,
    Warnings,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct EnabledSubtypes {
    inner: BitFlags<InternalModeSubtype>,
}

impl EnabledSubtypes {
    fn details(self) -> bool {
        self.inner.contains(InternalModeSubtype::Details)
    }
    fn warnings(self) -> bool {
        self.inner.contains(InternalModeSubtype::Warnings)
    }
}

impl From<ModeSubtype> for EnabledSubtypes {
    fn from(value: ModeSubtype) -> Self {
        match value {
            ModeSubtype::Details => Self {
                inner: InternalModeSubtype::Details.into(),
            },
            ModeSubtype::Warnings => Self {
                inner: InternalModeSubtype::Warnings.into(),
            },
        }
    }
}

impl IntoPropValue<EnabledSubtypes> for ModeSubtype {
    fn into_prop_value(self) -> EnabledSubtypes {
        self.into()
    }
}

impl BitOr for EnabledSubtypes {
    type Output = EnabledSubtypes;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            inner: self.inner | rhs.inner,
        }
    }
}

impl BitOr<ModeSubtype> for EnabledSubtypes {
    type Output = EnabledSubtypes;

    fn bitor(self, rhs: ModeSubtype) -> Self::Output {
        self | EnabledSubtypes::from(rhs)
    }
}

impl BitOr for ModeSubtype {
    type Output = EnabledSubtypes;

    fn bitor(self, rhs: Self) -> Self::Output {
        EnabledSubtypes::from(self) | EnabledSubtypes::from(rhs)
    }
}

impl BitOr<EnabledSubtypes> for ModeSubtype {
    type Output = EnabledSubtypes;

    fn bitor(self, rhs: EnabledSubtypes) -> Self::Output {
        rhs | self
    }
}

#[derive(Properties, PartialEq)]
pub struct TableModeSwitchProps {
    #[prop_or_default]
    pub entry_count: Option<usize>,

    pub types: EnabledSubtypes,
}

pub struct TableModeSwitch {
    current_mode: TableMode,

    set_titles_mode_cb: Callback<MouseEvent>,
    set_thumbs_mode_cb: Callback<MouseEvent>,
    set_warnings_received_mode_cb: Callback<MouseEvent>,
    set_warnings_issued_mode_cb: Callback<MouseEvent>,

    _location_handle: LocationHandle,
}

pub enum TableModeSwitchMessage {
    UpdateMode(TableMode),
    LocationUpdated(Location),
}

impl TableModeSwitch {
    fn verify_state(state: LocationState, ctx: &Context<Self>) -> LocationState {
        let props = ctx.props();
        let scope = ctx.link();
        match state.detail_table_mode {
            TableMode::Titles | TableMode::Thumbnails if props.types.details() => state,
            TableMode::WarningsReceived | TableMode::WarningsIssued if props.types.warnings() => {
                state
            }
            _ => {
                let state = LocationState {
                    detail_table_mode: if props.types.details() {
                        TableMode::Titles
                    } else {
                        TableMode::WarningsReceived
                    },
                    ..state
                };
                scope
                    .navigator()
                    .unwrap()
                    .replace_with_state(&scope.route::<MainRoute>().unwrap(), state);
                state
            }
        }
    }
}

impl Component for TableModeSwitch {
    type Properties = TableModeSwitchProps;
    type Message = TableModeSwitchMessage;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();

        let location_state = match scope.location().unwrap().state::<LocationState>() {
            Some(state) => Self::verify_state(*state, ctx),
            None => {
                let state = LocationState::default();
                scope
                    .navigator()
                    .unwrap()
                    .replace_with_state(&scope.route::<MainRoute>().unwrap(), state);
                state
            }
        };

        TableModeSwitch {
            current_mode: location_state.detail_table_mode,

            set_titles_mode_cb: scope
                .callback(|_| TableModeSwitchMessage::UpdateMode(TableMode::Titles)),
            set_thumbs_mode_cb: scope
                .callback(|_| TableModeSwitchMessage::UpdateMode(TableMode::Thumbnails)),
            set_warnings_received_mode_cb: scope
                .callback(|_| TableModeSwitchMessage::UpdateMode(TableMode::WarningsReceived)),
            set_warnings_issued_mode_cb: scope
                .callback(|_| TableModeSwitchMessage::UpdateMode(TableMode::WarningsIssued)),

            _location_handle: scope
                .add_location_listener(scope.callback(TableModeSwitchMessage::LocationUpdated))
                .unwrap(),
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="table-mode-switch">
                if ctx.props().types.details() {
                    <span class="table-mode button" onclick={&self.set_titles_mode_cb} selected={self.current_mode == TableMode::Titles}>{"Titles"}</span>
                    <span class="table-mode button" onclick={&self.set_thumbs_mode_cb} selected={self.current_mode == TableMode::Thumbnails}>{"Thumbnails"}</span>
                }
                if ctx.props().types.warnings() {
                    <span class="table-mode button" onclick={&self.set_warnings_received_mode_cb} selected={self.current_mode == TableMode::WarningsReceived}>{"Warnings received"}</span>
                    <span class="table-mode button" onclick={&self.set_warnings_issued_mode_cb} selected={self.current_mode == TableMode::WarningsIssued}>{"Warnings issued"}</span>
                }
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
        #[allow(clippy::match_same_arms)]
        match msg {
            TableModeSwitchMessage::LocationUpdated(location) => {
                let scope = ctx.link();
                let state = match location
                    .state::<LocationState>()
                    .or_else(|| scope.location().unwrap().state())
                {
                    Some(state) => Self::verify_state(*state, ctx),
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
            TableModeSwitchMessage::UpdateMode(new_mode) if new_mode != self.current_mode => {
                self.current_mode = new_mode;
                let scope = ctx.link();
                let mut state = scope
                    .location()
                    .unwrap()
                    .state::<LocationState>()
                    .as_deref()
                    .copied()
                    .unwrap_or_default();
                state.detail_table_mode = new_mode;
                scope
                    .navigator()
                    .unwrap()
                    .replace_with_state(&scope.route::<MainRoute>().unwrap(), state);
                true
            }
            TableModeSwitchMessage::UpdateMode(..) => false,
        }
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct PageSelectProps {
    pub page_count: usize,
}

#[function_component]
pub fn PageSelect(props: &PageSelectProps) -> Html {
    let state_handle = use_location_state();
    let state = state_handle.get_state();

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
            };
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
