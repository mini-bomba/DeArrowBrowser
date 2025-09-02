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

use serde::de::DeserializeOwned;
use yew::{BaseComponent, Html, Properties};

use crate::{settings::Settings, utils_app::RcEq};

#[derive(Properties)]
pub struct RowProps<T: TableRender> {
    pub items: RcEq<[T]>,
    pub index: usize,
    pub settings: T::Settings,
}

impl<T: TableRender> RowProps<T> {
    pub fn item(&self) -> &T {
        &self.items[self.index]
    }
}

impl<T: TableRender> PartialEq for RowProps<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items && self.index == other.index && self.settings == other.settings
    }
}

impl<T: TableRender> Eq for RowProps<T> {}

pub trait TableRender: Sized + DeserializeOwned + 'static {
    type Settings: Sized + Copy + Eq + Default + 'static;
    type RowRenderer: BaseComponent<Properties = RowProps<Self>>;
    const CLASS: &str;

    fn render_header(render_settings: Self::Settings, user_settings: &Settings) -> Html;
}
