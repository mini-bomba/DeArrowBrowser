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
use yew::prelude::*;
use yew_router::prelude::*;

use crate::MainRoute;
use super::icon::*;

pub fn videoid_link(video_id: AttrValue) -> Html {
    html! {
        <Link<MainRoute> to={MainRoute::Video { id: video_id }}>
            <Icon r#type={IconType::DABLogo} tooltip="View this video in DeArrow Browser"/>
        </Link<MainRoute>>
    }
}

pub fn userid_link(user_id: AttrValue) -> Html {
    html! {
        <Link<MainRoute> to={MainRoute::User { id: user_id }}>
            <Icon r#type={IconType::DABLogo} tooltip="View this user in DeArrow Browser"/>
        </Link<MainRoute>>
    }
}

pub fn uuid_link(uuid: AttrValue) -> Html {
    html! {
        <Link<MainRoute> to={MainRoute::UUID { id: uuid }}>
            <Icon r#type={IconType::DABLogo} tooltip="View this detail in DeArrow Browser"/>
        </Link<MainRoute>>
    }
}
