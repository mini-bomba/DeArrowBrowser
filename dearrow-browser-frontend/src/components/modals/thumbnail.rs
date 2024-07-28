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

use yew::prelude::*;

use std::rc::Rc;

use crate::thumbnails::components::{UnwrappedThumbnail, ThumbnailProps};

#[function_component]
pub fn ThumbnailModal(props: &ThumbnailProps) -> Html {
    let header_text: Rc<Rc<str>> = use_memo(props.clone(), |props| {
        match props.timestamp {
            None => format!("Video ID: {}, original thumbnail", props.video_id),
            Some(t) => format!("Video ID: {}, timestamp: {t}", props.video_id),
        }.into()
    });

    html!{
        <div id="thumbnail-modal">
            <h2>{"Thumbnail preview"}</h2>
            <h3>{(*header_text).clone()}</h3>
            <UnwrappedThumbnail ..props.clone() />
        </div>
    }
}
