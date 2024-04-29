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

use dearrow_browser_api::StatusResponse;
use reqwest::Url;
use yew::AttrValue;

pub use crate::components::modals::ModalRendererControls;

#[derive(Clone, PartialEq)]
pub struct WindowContext {
    pub origin: Url,
    pub logo_url: Option<AttrValue>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct UpdateClock(pub bool);

pub type StatusContext = Option<Rc<StatusResponse>>;
