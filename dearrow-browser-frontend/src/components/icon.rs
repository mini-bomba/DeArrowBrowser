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
    UpvoteAndLock,
    DownvoteAndRemove,
    Wait,
    Done,
    Close,
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
        IconType::DABLogo           => classes!("icon", "icon-dablogo"),
        IconType::Downvote          => classes!("icon", "icon-downvote"),
        IconType::Locked            => classes!("icon", "icon-locked"),
        IconType::Original          => classes!("icon", "icon-original"),
        IconType::PartiallyHidden   => classes!("icon", "icon-downvote", "grayscale"),
        IconType::Removed           => classes!("icon", "icon-removed"),
        IconType::Replaced          => classes!("icon", "icon-replaced"),
        IconType::Settings          => classes!("icon", "icon-settings"),
        IconType::ShadowHidden      => classes!("icon", "icon-shadowhidden"),
        IconType::TimestampMissing  => classes!("icon", "icon-timestamp-missing"),
        IconType::Unverified        => classes!("icon", "icon-unverified"),
        IconType::Upvote            => classes!("icon", "icon-upvote"),
        IconType::VIP               => classes!("icon", "icon-vip"),
        IconType::VotesMissing      => classes!("icon", "icon-votes-missing"),
        IconType::UpvoteAndLock     => classes!("icon", "icon-upvote-and-lock"),
        IconType::DownvoteAndRemove => classes!("icon", "icon-downvote-and-remove"),
        IconType::Wait              => classes!("icon", "icon-wait"),
        IconType::Done              => classes!("icon", "icon-done"),
        IconType::Close             => classes!("icon", "icon-removed", "grayscale"),
    };

    html! {
        <span {class} title={props.tooltip.clone()}></span>
    }
}
