/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2025 mini_bomba
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

use yew::{Html, html};

use crate::utils_app::html_length;

/// This is for the compact mode, to avoid a trailing bar
///
/// Yes, this is stupid
/// Let me do stupid things
/// I'm having *fun*
pub fn bar_prepender_if_not_empty(html: Html) -> Html {
    if html_length(&html) == 0 {
        html! {}
    } else {
        html! {
            <>
                {" | "}
                {html}
            </>
        }
    }
}

#[macro_export]
macro_rules! score_col {
    ($flags_func:ident, $detail:expr, $expanded:expr) => {
        if $detail.votes_missing {
            html! {
                <>
                    <em>{"No data"}</em>
                    if $expanded {<br />} else {{" | "}}
                    {$flags_func($detail)}
                </>
            }
        } else if $expanded {
            html! {
                <>
                    <span class="hoverswitch">
                        <span>{$detail.score}</span>
                        <span><Icon r#type={IconType::Upvote} />{format!(" {} | {} ", $detail.votes, $detail.downvotes)}<Icon r#type={IconType::Downvote} /></span>
                    </span>
                    <br />
                    {$flags_func($detail)}
                </>
            }

        } else {
            html! {
                <>
                    {format!("{} | ", $detail.score)}<Icon r#type={IconType::Upvote} />{format!(" {} | {} ", $detail.votes, $detail.downvotes)}<Icon r#type={IconType::Downvote} />
                    {$crate::components::tables::macros::bar_prepender_if_not_empty($flags_func($detail))}
                </>
            }
        }
    };
}

#[macro_export]
macro_rules! uuid_cell {
    ($uuid:expr, $multiline:expr) => {
        html! {
            <>
                {$uuid.clone()}
                if $multiline { <br /> } else {{" "}}
                {$crate::components::links::uuid_link($uuid.clone().into())}
            </>
        }
    };
}

#[macro_export]
macro_rules! userid_cell {
    ($userid:expr, $rows:expr, $multiline:expr) => {
        html! {
            <>
                <textarea readonly=true cols=16 rows={$rows} ~value={$userid.clone()} />
                if $multiline { <br /> } else {{" "}}
                {$crate::components::links::userid_link($userid.clone().into())}
            </>
        }
    };
}

#[macro_export]
macro_rules! username_cell {
    ($username:expr, $rows:expr) => {
        if let Some(ref name) = $username {
            html! {<textarea readonly=true cols=16 rows={$rows} ~value={name.to_string()} />}
        } else {
            html! {{"-"}}
        }
    };
}
