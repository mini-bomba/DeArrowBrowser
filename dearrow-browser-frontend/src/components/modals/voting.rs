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

use std::rc::Rc;

use futures::FutureExt;
use gloo_console::{error, warn};
use reqwest::Url;
use yew::prelude::*;
use dearrow_browser_api::unsync::{ApiThumbnail, ApiTitle};

use crate::components::async_task_manager::{AsyncTaskFuture, AsyncTaskResult};
use crate::components::icon::{Icon, IconType};
use crate::constants::{REQWEST_CLIENT, SBS_BRANDING_ENDPOINT, USER_AGENT};
use crate::contexts::{AsyncTaskControl, ModalRendererControls, ModalMessage, SettingsContext, UserContext};
use crate::sbserver::{PostBrandingBody, SBServerThumbnail, SBServerTitle};
use crate::thumbnails::components::{Thumbnail, ThumbnailCaption};
use crate::utils::ReqwestUrlExt;


#[derive(Clone, PartialEq, Properties)]
pub struct VotingParams {
    pub detail: VotingDetail,
}

#[derive(Clone, PartialEq)]
pub enum VotingDetail {
    Thumbnail(ApiThumbnail),
    Title(ApiTitle),
}

#[derive(Clone, Copy)]
struct VotingMode {
    downvote: bool,
    auto_lock: bool,
}

fn create_voting_task(url: Url, body: PostBrandingBody) -> (AsyncTaskFuture, Html) {
    (async move {
        let resp = REQWEST_CLIENT.post(url.clone())
            .json(&body)
            .send().await;
        match resp {
            Err(e) => {
                error!(format!("Failed to send voting request: {e:?}"));
                (
                    AsyncTaskResult::DismissOrRetry { success: false, retry: Box::new(move || create_voting_task(url, body)) },
                    html! {{"Failed to send request. Check console for details."}}
                )
            },
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    (
                        AsyncTaskResult::AutoDismiss { success: true },
                        html! {{format!("Upvoted successfully! (status code {status})")}},
                    )
                } else {
                    match resp.text().await {
                        Ok(text) => {
                            error!(format!("Failed to send vote: status code {status}, response body:\n{text}"));
                            (
                                AsyncTaskResult::DismissOrRetry { success: false, retry: Box::new(move || create_voting_task(url, body)) },
                                html! {{format!("Failed to vote. Check console for details. (status code {status})")}}
                            )
                        },
                        Err(..) => {
                            error!(format!("Failed to send vote: status code {status}, could not decode body"));
                            (
                                AsyncTaskResult::DismissOrRetry { success: false, retry: Box::new(move || create_voting_task(url, body)) },
                                html! {{format!("Failed to vote. Check console for details. (status code {status})")}}
                            )
                        },
                    }
                }
            }
        }
        
    }.boxed_local(), html! {{"Sending request..."}})
}

fn voting_callback(async_task_control: AsyncTaskControl, modal_control: ModalRendererControls, voting_mode: VotingMode) 
    -> impl Fn(MouseEvent, &(VotingDetail, Option<Rc<str>>, Rc<str>)) 
{
    move |_, (detail, user_id, sbs_base)| {
        let Some(user_id) = user_id.clone() else { return; };
        let url = match Url::parse(sbs_base) {
            Err(e) => {
                warn!(format!("Failed to parse SBServer base URL: {e:?}"));
                return;
            },
            Ok(mut url) => {
                if url.extend_segments(SBS_BRANDING_ENDPOINT).is_err() {
                    warn!("Failed to construct voting URL: SBServer base URL was not a vaild base!");
                    return;
                };
                url
            }
        };
        let (body, task_name) = match detail {
            VotingDetail::Title(ref title) => (PostBrandingBody {
                video_id: title.video_id.clone(),
                user_id,
                user_agent: *USER_AGENT,
                service: "YouTube",
                title: Some(SBServerTitle {
                    title: title.title.clone(),
                }),
                thumbnail: None,
                downvote: voting_mode.downvote,
                auto_lock: voting_mode.auto_lock,
            }, match voting_mode {
                VotingMode { downvote: false, auto_lock: false } => format!("Upvoting title {}",   title.uuid),
                VotingMode { downvote: true,  auto_lock: false } => format!("Downvoting title {}", title.uuid),
                VotingMode { downvote: false, auto_lock: true  } => format!("Locking title {}",    title.uuid),
                VotingMode { downvote: true,  auto_lock: true  } => format!("Removing title {}",   title.uuid),
            }),
            VotingDetail::Thumbnail(ref thumb) => (PostBrandingBody {
                video_id: thumb.video_id.clone(),
                user_id,
                user_agent: *USER_AGENT,
                service: "YouTube",
                title: None,
                thumbnail: Some(SBServerThumbnail {
                    timestamp: thumb.timestamp,
                    original: thumb.original,
                }),
                downvote: voting_mode.downvote,
                auto_lock: voting_mode.auto_lock,
            }, match voting_mode {
                VotingMode { downvote: false, auto_lock: false } => format!("Upvoting thumbnail {}",   thumb.uuid),
                VotingMode { downvote: true,  auto_lock: false } => format!("Downvoting thumbnail {}", thumb.uuid),
                VotingMode { downvote: false, auto_lock: true  } => format!("Locking thumbnail {}",    thumb.uuid),
                VotingMode { downvote: true,  auto_lock: true  } => format!("Removing thumbnail {}",   thumb.uuid),
            }),
        };
        let (task, summary) = create_voting_task(url, body);
        async_task_control.submit_task(task_name.into(), summary, task);
        modal_control.emit(ModalMessage::CloseTop);
        
    }
}

#[function_component]
pub fn VotingModal(params: &VotingParams) -> Html {
    let user_context: UserContext = use_context().unwrap();
    let async_task_control: AsyncTaskControl = use_context().unwrap();
    let modal_control: ModalRendererControls = use_context().unwrap();
    let settings_context: SettingsContext = use_context().unwrap();
    let settings = settings_context.settings();
    let is_vip = user_context.as_ref().is_some_and(|user| user.data.as_ref().is_some_and(|user| user.as_ref().is_ok_and(|user| user.vip)));

    let upvote = use_callback(
        (params.detail.clone(), settings.private_user_id.clone(), settings.sponsorblock_api_base_url.clone()), 
        voting_callback(async_task_control.clone(), modal_control.clone(), VotingMode { downvote: false, auto_lock: false })
    );
    let downvote = use_callback(
        (params.detail.clone(), settings.private_user_id.clone(), settings.sponsorblock_api_base_url.clone()), 
        voting_callback(async_task_control.clone(), modal_control.clone(), VotingMode { downvote: true, auto_lock: false })
    );
    let upvote_lock = use_callback(
        (params.detail.clone(), settings.private_user_id.clone(), settings.sponsorblock_api_base_url.clone()), 
        voting_callback(async_task_control.clone(), modal_control.clone(), VotingMode { downvote: false, auto_lock: true })
    );
    let downvote_remove = use_callback(
        (params.detail.clone(), settings.private_user_id.clone(), settings.sponsorblock_api_base_url.clone()), 
        voting_callback(async_task_control.clone(), modal_control.clone(), VotingMode { downvote: true, auto_lock: true })
    );

    let vote_buttons = html! {
        <table>
            <tr>
                if is_vip {
                    <th />
                }
                <th>{"Upvote"}</th>
                <th>{"Downvote"}</th>
            </tr>
            <tr>
                if is_vip {
                    <th>{"User"}</th>
                }
                <td class="clickable" onclick={upvote}><Icon r#type={IconType::Upvote} tooltip={"Upvote"} /></td>
                <td class="clickable" onclick={downvote}><Icon r#type={IconType::Downvote} tooltip={"Downvote"} /></td>
            </tr>
            if is_vip {
                <tr>
                    <th>{"VIP"}</th>
                    <td class="clickable" onclick={upvote_lock}><Icon r#type={IconType::UpvoteAndLock} tooltip={"Upvote and lock"} /></td>
                    <td class="clickable" onclick={downvote_remove}><Icon r#type={IconType::DownvoteAndRemove} tooltip={"Downvote and remove"} /></td>
                </tr>
            }
        </table>
    };
    match &params.detail {
        VotingDetail::Thumbnail(thumbnail) => {
            html! {
                <div id="voting-modal">
                    <h2>{"Voting on thumbnail "}{thumbnail.uuid.clone()}</h2>
                    <Thumbnail
                        video_id={thumbnail.video_id.clone()}
                        timestamp={thumbnail.timestamp}
                        caption={ThumbnailCaption::Text(thumbnail.timestamp.map_or("Original thumbnail".into(), |t| t.to_string().into()))}
                    />
                    {vote_buttons}
                </div>
            }
        },
        VotingDetail::Title(title) => {
            html! {
                <div id="voting-modal">
                    <h2>{"Voting on title "}{title.uuid.clone()}</h2>
                    <span>{title.title.clone()}</span>
                    {vote_buttons}
                </div>
            }
        }
    }
}
