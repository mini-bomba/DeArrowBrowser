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

use std::{num::NonZeroUsize, rc::Rc};

use chrono::DateTime;
use dearrow_browser_api::unsync::{ApiWarning, Extension};
use error_handling::ErrorContext;
use reqwest::Url;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{
    components::{links::userid_link, tables::switch::PageSelect},
    contexts::SettingsContext,
    pages::LocationState,
    utils::{api_request, render_datetime},
};

#[derive(Properties, PartialEq, Clone)]
struct WarningRowProps {
    pub warnings: Rc<[ApiWarning]>,
    pub index: usize,

    #[prop_or_default]
    pub hide_issuer: bool,
    #[prop_or_default]
    pub hide_receiver: bool,
}

#[function_component]
fn WarningRow(props: &WarningRowProps) -> Html {
    let warning = &props.warnings[props.index];
    let timestamp = use_memo(warning.time_issued, |timestamp| {
        DateTime::from_timestamp_millis(*timestamp)
            .map_or_else(|| timestamp.to_string(), render_datetime)
    });
    let extension = match warning.extension {
        Extension::DeArrow => "for DeArrow",
        Extension::SponsorBlock => "for SponsorBlock",
    };
    let status = if warning.active {
        "Active"
    } else {
        "Acknowledged"
    };
    html! {
        <tr>
            <td>
                {timestamp}<br/>
                {extension}<br/>
                {status}
            </td>
            <td class="warning-message-col"><pre>{warning.message.clone()}</pre></td>
            if !props.hide_issuer {
                <td>
                    <textarea readonly=true cols=20 rows=3 ~value={warning.issuer_user_id.clone()} /><br/>
                    if let Some(username) = warning.issuer_username.clone() {
                        <textarea readonly=true cols=20 rows=3 ~value={username} /><br/>
                    }
                    {userid_link(warning.issuer_user_id.clone().into())}
                </td>
            }
            if !props.hide_receiver {
                <td>
                    <textarea readonly=true cols=20 rows=3 ~value={warning.warned_user_id.clone()} /><br/>
                    if let Some(username) = warning.warned_username.clone() {
                        <textarea readonly=true cols=20 rows=3 ~value={username} /><br/>
                    }
                    {userid_link(warning.warned_user_id.clone().into())}
                </td>
            }
        </tr>
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct BaseWarningsTableProps {
    pub warnings: Rc<[ApiWarning]>,

    #[prop_or_default]
    pub hide_issuer: bool,
    #[prop_or_default]
    pub hide_receiver: bool,
}

#[function_component]
pub fn BaseWarningsTable(props: &BaseWarningsTableProps) -> Html {
    let row_props = WarningRowProps {
        warnings: props.warnings.clone(),
        index: 0,
        hide_issuer: props.hide_issuer,
        hide_receiver: props.hide_receiver,
    };
    html! {
        <table class="warning-table">
            <tr class="header">
                <th>{"Issued"}</th>
                <th>{"Message"}</th>
                if !props.hide_issuer {
                    <th>{"Issuer"}</th>
                }
                if !props.hide_receiver {
                    <th>{"Receiver"}</th>
                }
            </tr>
            { for props.warnings.iter().enumerate().map(|(i, t)| {
                let mut row_props = row_props.clone();
                row_props.index = i;
                html! { <WarningRow key={t.time_issued} ..row_props />}
            }) }
        </table>
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct WarningsTableProps {
    pub url: Rc<Url>,

    #[prop_or_default]
    pub entry_count: Option<UseStateSetter<Option<usize>>>,

    #[prop_or_default]
    pub hide_issuer: bool,
    #[prop_or_default]
    pub hide_receiver: bool,
}

#[derive(Clone)]
struct PaginatedWarnings {
    full: Rc<[ApiWarning]>,
    page: Rc<[ApiWarning]>,
}

pub struct PaginatedWarningsTable {
    warnings: Option<Result<PaginatedWarnings, ErrorContext>>,
    entries_per_page: NonZeroUsize,
    current_page: usize,

    _settings_context_handle: ContextHandle<SettingsContext>,
    _location_handle: LocationHandle,
}

pub enum PaginatedWarningsTableMessage {
    WarningsFetched {
        warnings: Result<Rc<[ApiWarning]>, ErrorContext>,
        url: Rc<Url>,
    },
    SettingsUpdated {
        entries_per_page: NonZeroUsize,
    },
    LocationStateUpdated {
        current_page: usize,
    },
}

impl PaginatedWarningsTable {
    async fn download_warnings(url: Rc<Url>) -> PaginatedWarningsTableMessage {
        PaginatedWarningsTableMessage::WarningsFetched {
            url: url.clone(),
            warnings: async move {
                let mut warnings: Rc<[ApiWarning]> = api_request((*url).clone()).await?;
                Rc::get_mut(&mut warnings)
                    .expect("Should be able to get a mutable reference")
                    .sort_unstable_by(|a, b| a.time_issued.cmp(&b.time_issued).reverse());
                Ok(warnings)
            }
            .await,
        }
    }
}

impl Component for PaginatedWarningsTable {
    type Properties = WarningsTableProps;
    type Message = PaginatedWarningsTableMessage;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();
        let props = ctx.props();
        scope.send_future(Self::download_warnings(props.url.clone()));
        let (settings, settings_context_handle) = scope
            .context(scope.callback(|settings: SettingsContext| {
                PaginatedWarningsTableMessage::SettingsUpdated {
                    entries_per_page: settings.settings().entries_per_page,
                }
            }))
            .expect("SettingsContext should be avaialble");
        let current_page = scope
            .location()
            .expect("Location should be available")
            .state::<LocationState>()
            .unwrap_or_default()
            .detail_table_page;

        if let Some(handle) = &props.entry_count {
            handle.set(None);
        }

        Self {
            warnings: None,
            entries_per_page: settings.settings().entries_per_page,
            current_page,

            _settings_context_handle: settings_context_handle,
            _location_handle: scope
                .add_location_listener(scope.callback(|location: Location| {
                    PaginatedWarningsTableMessage::LocationStateUpdated {
                        current_page: location
                            .state::<LocationState>()
                            .unwrap_or_default()
                            .detail_table_page,
                    }
                }))
                .expect("Location should be available"),
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        match &self.warnings {
            None => html! {<center><b>{"Loading..."}</b></center>},
            Some(Err(error)) => html! {
                <center>
                    <b>{"Failed to fetch details from the API :/"}</b>
                    <pre>{format!("{error:?}")}</pre>
                </center>
            },
            Some(Ok(warnings)) => {
                let page_count = (warnings.full.len() / self.entries_per_page) + 1;
                let props = ctx.props();
                html! {
                    <>
                        <BaseWarningsTable warnings={warnings.page.clone()} hide_issuer={props.hide_issuer} hide_receiver={props.hide_receiver} />
                        if page_count > 1 {
                            <PageSelect {page_count} />
                        }
                    </>
                }
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        let new_props = ctx.props();
        if new_props.url != old_props.url {
            self.warnings = None;
            if let Some(handle) = &new_props.entry_count {
                handle.set(None);
            }
            ctx.link()
                .send_future(Self::download_warnings(new_props.url.clone()));
        }
        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            PaginatedWarningsTableMessage::SettingsUpdated { entries_per_page } => {
                if entries_per_page == self.entries_per_page {
                    false
                } else {
                    self.entries_per_page = entries_per_page;
                    true
                }
            }
            PaginatedWarningsTableMessage::WarningsFetched { warnings, url } => {
                let props = ctx.props();
                if url != props.url {
                    return false;
                }
                self.warnings = Some(warnings.map(|warnings| {
                    if let Some(handle) = &props.entry_count {
                        handle.set(Some(warnings.len()));
                    }
                    let entries: usize = self.entries_per_page.into();
                    let page: usize = self.current_page;
                    if warnings.len() >= entries {
                        PaginatedWarnings {
                            full: warnings.clone(),
                            page: warnings,
                        }
                    } else if warnings.len() <= (page + 1) * entries {
                        PaginatedWarnings {
                            page: warnings.get(page * entries..).unwrap_or_default().into(),
                            full: warnings,
                        }
                    } else {
                        PaginatedWarnings {
                            page: warnings
                                .get(page * entries..(page + 1) * entries)
                                .unwrap_or_default()
                                .into(),
                            full: warnings,
                        }
                    }
                }));
                true
            }
            PaginatedWarningsTableMessage::LocationStateUpdated { current_page } => {
                if current_page == self.current_page {
                    false
                } else {
                    self.current_page = current_page;
                    true
                }
            }
        }
    }
}
