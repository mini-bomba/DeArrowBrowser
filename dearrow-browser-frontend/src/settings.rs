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

use serde::{Deserialize, Serialize};
use strum::{EnumString, EnumVariantNames, IntoStaticStr};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Settings {
    pub thumbgen_api_base_url: Rc<str>,
    pub entries_per_page: NonZeroUsize,
    pub title_table_layout: TableLayout,
    pub thumbnail_table_layout: TableLayout,
    pub render_thumbnails_in_tables: bool,
    pub disable_sharedworker: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            thumbgen_api_base_url: "https://dearrow-thumb.minibomba.pro/".into(),
            entries_per_page: 50.try_into().unwrap(),
            title_table_layout: TableLayout::Expanded,
            thumbnail_table_layout: TableLayout::Expanded,
            render_thumbnails_in_tables: false,
            disable_sharedworker: false,
        }
    }
}

// serde names set explicitly to avoid issues in the future if names changes
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, IntoStaticStr, EnumVariantNames, Debug)]
pub enum TableLayout {
    #[serde(rename="compressed")]
    Compressed,
    #[serde(rename="compact")]
    Compact,
    #[serde(rename="expanded", other)]
    Expanded,
}
