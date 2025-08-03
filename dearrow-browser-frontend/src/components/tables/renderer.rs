/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2025 mini_bomba
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

use yew::{classes, function_component, html, use_context, AttrValue, Html, Properties};

use crate::{
    components::tables::{switch::{PageSelect, Tabs}, r#trait::TableRender},
    contexts::SettingsContext,
    hooks::use_location_state,
    utils::RcEq,
};

#[derive(Properties)]
pub struct TableRendererProps<T: TableRender> {
    pub items: RcEq<[T]>,
    #[prop_or(0)]
    pub start: usize,
    #[prop_or(usize::MAX)]
    pub count: usize,
    #[prop_or_default]
    pub settings: T::Settings,
}

impl<T: TableRender> PartialEq for TableRendererProps<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
            && self.start == other.start
            && self.count == other.count
            && self.settings == other.settings
    }
}

impl<T: TableRender> Eq for TableRendererProps<T> {}

#[function_component]
pub fn TableRenderer<T: TableRender>(props: &TableRendererProps<T>) -> Html {
    let settings_context: SettingsContext =
        use_context().expect("SettingsContext should be available");
    let settings = settings_context.settings();

    let header_classes = classes!("header", settings.sticky_headers.then_some("sticky"));

    html! {
        <table class={classes!("detail-table", T::CLASS)} data-layout={AttrValue::Static(settings.title_table_layout.into())}>
            <tr class={header_classes}>
                {T::render_header(props.settings, settings)}
            </tr>
            {
                for props.items.iter()
                    .enumerate()
                    .skip(props.start)
                    .take(props.count)
                    .map(|(index, _)| html! {
                        <tr>
                            <T::RowRenderer items={props.items.clone()} {index} settings={props.settings} />
                        </tr>
                    })
            }
        </table>
    }
}

#[derive(Properties)]
pub struct PaginatedTableRendererProps<T: TableRender> {
    pub items: RcEq<[T]>,
    #[prop_or_default]
    pub settings: T::Settings,
}

impl<T: TableRender> PartialEq for PaginatedTableRendererProps<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items && self.settings == other.settings
    }
}

impl<T: TableRender> Eq for PaginatedTableRendererProps<T> {}

#[function_component]
pub fn PaginatedTableRenderer<T: TableRender, S: Tabs>(props: &PaginatedTableRendererProps<T>) -> Html {
    let state = use_location_state().get_state::<S>();
    let settings_context: SettingsContext =
        use_context().expect("SettingsContext should be available");
    let settings = settings_context.settings();

    let entries_per_page = settings.entries_per_page.get();
    let page_count = props.items.len().div_ceil(entries_per_page);

    html! {<>
        <TableRenderer<T>
            items={props.items.clone()}
            settings={props.settings}
            count={entries_per_page}
            start={state.page*entries_per_page}
        />
        if page_count > 1 {
            <PageSelect<S> {page_count} />
        }
    </>}
}
