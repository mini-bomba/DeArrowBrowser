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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconType {
    DABLogo,
    Downvote,
    Locked,
    Original,
    PartiallyHidden,
    Removed,
    Replaced,
    Settings,
    ShadowHidden,
    TimestampMissing,
    Unverified,
    Upvote,
    VIP,
    VotesMissing,
}

#[derive(Properties, PartialEq)]
pub struct IconProps {
    pub r#type: IconType,
    #[prop_or_default]
    pub tooltip: Option<AttrValue>,
}

#[function_component]
pub fn Icon(props: &IconProps) -> Html {
    let class = match props.r#type {
        IconType::DABLogo => classes!("icon-dablogo"),
        IconType::Downvote => classes!("icon-downvote"),
        IconType::Locked => classes!("icon-locked"),
        IconType::Original => classes!("icon-original"),
        IconType::PartiallyHidden => classes!("icon-downvote", "grayscale"),
        IconType::Removed => classes!("icon-removed"),
        IconType::Replaced => classes!("icon-replaced"),
        IconType::Settings => classes!("icon-settings"),
        IconType::ShadowHidden => classes!("icon-shadowhidden"),
        IconType::TimestampMissing => classes!("icon-timestamp-missing"),
        IconType::Unverified => classes!("icon-unverified"),
        IconType::Upvote => classes!("icon-upvote"),
        IconType::VIP => classes!("icon-vip"),
        IconType::VotesMissing => classes!("icon-votes-missing"),
    };

    html! {
        <span class={classes!("icon", class)} title={props.tooltip.clone()}></span>
    }
}
