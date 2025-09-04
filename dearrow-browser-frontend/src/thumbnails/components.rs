/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024-2025 mini_bomba
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

use gloo_console::{error, log};
use reqwest::Url;
use yew::html::ChildrenProps;
use yew::prelude::*;

use crate::components::modals::{thumbnail::ThumbnailModal, ModalMessage};
use crate::hooks::use_async_suspension;
use crate::worker_api::{WorkerRequest, WorkerSetting};
use crate::worker_client::{Error, WorkerState};
use crate::{ModalRendererControls, SettingsContext};

use super::common::{ThumbgenStats, ThumbnailKey};
use super::local::{LocalBlobLink, LocalThumbGenerator};
use super::remote::{RemoteBlobLink, RemoteThumbnailGenerator};

#[derive(Clone, Eq, PartialEq)]
pub enum Thumbgen {
    Remote(RemoteThumbnailGenerator),
    Local(LocalThumbGenerator),
}

#[derive(Clone, Eq, PartialEq)]
pub enum ThumbnailUrl {
    Local(Rc<LocalBlobLink>),
    Remote(Rc<RemoteBlobLink>),
}

impl ThumbnailUrl {
    fn get_url(&self) -> &str {
        match self {
            Self::Local(url) => url.inner_url(),
            Self::Remote(url) => url.inner_url(),
        }
    }
}

impl Thumbgen {
    fn from_worker_state(worker_state: WorkerState) -> Option<Self> {
        match worker_state {
            WorkerState::Loading => None,
            WorkerState::Ready(client) => Some(Self::Remote(RemoteThumbnailGenerator { client })),
            WorkerState::Failed(..) => Some(Self::Local(LocalThumbGenerator::new())),
        }
    }

    fn update_url(&self, new_url: &str) {
        match self {
            Thumbgen::Remote(worker) => {
                if let Err(e) = worker.client.post_request(WorkerRequest::SettingUpdated {
                    setting: WorkerSetting::ThumbgenBaseUrl(new_url.to_string())
                }) {
                    error!(format!("Failed to notify Thumbgen::Remote about a thumbgen API base URL change: {e}"));
                }
            },
            Thumbgen::Local(gen) => {
                let mut url = match Url::parse(new_url) {
                    Ok(url) => url,
                    Err(e) => return error!(format!("Failed to parse new ThumbgenBaseUrl: {e}")),
                };
                {
                    let Ok(mut path) = url.path_segments_mut() else {
                        return error!(format!("Failed to append API endpoint to new ThumbgenBaseUrl: {new_url} cannot be a base"))
                    };
                    path.extend(&["api", "v1", "getThumbnail"]);
                };
                gen.set_api_url(url);
                let errors_removed = gen.clear_errors();
                log!(format!("Cleared {errors_removed} error entries after updating thumbgen API URL"));
            }
        }
    }

    pub async fn get_thumbnail(&self, key: &ThumbnailKey) -> Result<ThumbnailUrl, Error> {
        match self {
            Self::Remote(worker) => worker.get_thumbnail(key.clone()).await.map(|t| ThumbnailUrl::Remote(Rc::new(t))),
            Self::Local(gen) => gen.get_thumb(key).await.map(ThumbnailUrl::Local).map_err(|e| Error::Thumbgen(e.into())),
        }
    }

    pub async fn get_stats(&self) -> Result<ThumbgenStats, Error> {
        match self {
            Self::Remote(worker) => worker.get_stats().await,
            Self::Local(gen) => Ok(gen.get_stats()),
        }
    }

    pub fn clear_errors(&self) {
        match self {
            Self::Remote(worker) => worker.clear_errors(),
            Self::Local(gen) => drop(gen.clear_errors()),
        }
    }
}

pub type ThumbgenContext = Option<Thumbgen>;
pub trait ThumbgenContextExt {
    fn get_status(&self) -> &'static str;
}

impl ThumbgenContextExt for ThumbgenContext {
    fn get_status(&self) -> &'static str {
        match self {
            None => "Initializing",
            Some(Thumbgen::Local {..}) => "Ready",
            Some(Thumbgen::Remote(ref gen)) => {
                if gen.client.is_protocol_mismatched() {
                    "Ready (mismatched protocol)"
                } else {
                    "Ready"
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ThumbgenRefreshContext {
    pub value: u8,
    bump_callback: Callback<()>,
}

impl ThumbgenRefreshContext {
    pub fn trigger_refresh(&self) {
        self.bump_callback.emit(());
    }
}

#[derive(Properties, PartialEq)]
pub struct ThumbnailGeneratorProviderProps {
    pub children: Html
}

pub struct ThumbgenProvider {
    refresh_ctx: ThumbgenRefreshContext,
    thumbgen: Option<Thumbgen>,
    thumbgen_url: Rc<str>,

    _worker_context: ContextHandle<WorkerState>,
    _settings_context: ContextHandle<SettingsContext>,
}

pub enum ThumbgenProviderMessage {
    WorkerStateUpdate(WorkerState),
    ThumbgenUrlUpdate(Rc<str>),
    BumpRefreshIdx,
}

impl Component for ThumbgenProvider {
    type Properties = ChildrenProps;
    type Message = ThumbgenProviderMessage;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();
        let (worker_state, worker_context) = scope
            .context::<WorkerState>(scope.callback(ThumbgenProviderMessage::WorkerStateUpdate))
            .expect("Worker client should be available");
        let (settings, settings_context) = scope
            .context::<SettingsContext>(scope.callback(|s: SettingsContext| {
                ThumbgenProviderMessage::ThumbgenUrlUpdate(
                    s.settings().thumbgen_api_base_url.clone(),
                )
            }))
            .expect("Settings should be available");

        let this = Self {
            thumbgen: Thumbgen::from_worker_state(worker_state),
            refresh_ctx: ThumbgenRefreshContext {
                value: 0,
                bump_callback: scope.callback(|()| ThumbgenProviderMessage::BumpRefreshIdx),
            },
            thumbgen_url: settings.settings().thumbgen_api_base_url.clone(),

            _worker_context: worker_context,
            _settings_context: settings_context,
        };
        if let Some(thumbgen) = &this.thumbgen {
            thumbgen.update_url(&this.thumbgen_url);
        }
        this
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <ContextProvider<ThumbgenContext> context={self.thumbgen.clone()}>
            <ContextProvider<ThumbgenRefreshContext> context={self.refresh_ctx.clone()}>
                { ctx.props().children.clone() }
            </ContextProvider<ThumbgenRefreshContext>>
            </ContextProvider<ThumbgenContext>>
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ThumbgenProviderMessage::BumpRefreshIdx => {
                self.refresh_ctx.value = self.refresh_ctx.value.wrapping_add(1);
                true
            },
            ThumbgenProviderMessage::WorkerStateUpdate(new_state) => {
                self.thumbgen = Thumbgen::from_worker_state(new_state);
                if let Some(thumbgen) = &self.thumbgen {
                    thumbgen.update_url(&self.thumbgen_url);
                }
                self.refresh_ctx.value = self.refresh_ctx.value.wrapping_add(1);
                true
            },
            ThumbgenProviderMessage::ThumbgenUrlUpdate(new_url) => {
                if self.thumbgen_url == new_url {
                    return false;
                }
                self.thumbgen_url = new_url;
                if let Some(thumbgen) = &self.thumbgen {
                    thumbgen.update_url(&self.thumbgen_url);
                }
                self.refresh_ctx.value = self.refresh_ctx.value.wrapping_add(1);
                true
            }
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct BaseThumbnailProps {
    pub thumb_key: ThumbnailKey,
}

#[function_component]
pub fn BaseThumbnail(props: &BaseThumbnailProps) -> HtmlResult {
    let generator: ThumbgenContext = use_context().expect("BaseThumbnail must be run under a ThumbnailGeneratorProvider");
    let refresher: ThumbgenRefreshContext = use_context().expect("BaseThumbnail must be run under a ThumbnailGeneratorProvider");
    let thumbnail = use_async_suspension(|(generator, key, _)| async move {
        Some(generator?.get_thumbnail(&key).await)
    }, (generator, props.thumb_key.clone(), refresher))?;
    
    Ok(match *thumbnail {
        None => html! { <span class="thumbnail-error">{"Waiting for thumbnail generator..."}</span>},
        Some(Err(ref err)) => html! { <span class="thumbnail-error">{format!("{err:?}")}</span> },
        Some(Ok(ref url)) => html! { <img class="thumbnail" src={Rc::from(url.get_url())} /> },
    })
}

#[derive(Properties, PartialEq, Clone)]
pub struct UnwrappedThumbnailProps {
    pub video_id: Rc<str>,
    /// none means original thumb
    pub timestamp: Option<f64>,
}

#[function_component]
pub fn UnwrappedThumbnail(props: &UnwrappedThumbnailProps) -> Html {
    let timestamp: Rc<Option<Rc<str>>> = use_memo(props.clone(), |props| {
        props.timestamp.map(|t| t.to_string().into())
    });
    let fallback = html! {<span class="thumbnail-error">{"Generating thumbnail..."}</span>};
    let thumb_key = ThumbnailKey {
        video_id: props.video_id.clone(),
        timestamp: (*timestamp).clone(),
    };
    html! {
        <Suspense {fallback}>
            <BaseThumbnail {thumb_key} />
        </Suspense>
    }
}

#[derive(PartialEq, Clone, Default)]
pub enum ThumbnailCaption {
    #[default]
    None,
    Text(AttrValue),
    Html(Html),
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
#[allow(non_camel_case_types)]
pub enum ContainerType {
    #[default]
    div,
    td,
}

#[derive(Properties, PartialEq, Clone)]
pub struct WrappedThumbnailProps {
    pub video_id: Rc<str>,
    /// none means original thumb
    pub timestamp: Option<f64>,
    /// displayed at the bottom of the image
    #[prop_or_default]
    pub caption: ThumbnailCaption,
    #[prop_or_default]
    pub container_type: ContainerType,
}

#[function_component]
pub fn Thumbnail(props: &WrappedThumbnailProps) -> Html {
    let modal_controls: ModalRendererControls = use_context().expect("ModalRendererControls should be available");
    let unwrapped_props = UnwrappedThumbnailProps {
        video_id: props.video_id.clone(),
        timestamp: props.timestamp,
    };
    let onclick = {
        let props = unwrapped_props.clone();
        Callback::from(move |_| {
            modal_controls.emit(ModalMessage::Open(html! {
                <ThumbnailModal ..props.clone() />
            }));
        })
    };
    let content = html! {
        <>
            <UnwrappedThumbnail ..unwrapped_props.clone() />
            if let ThumbnailCaption::Text(caption) = &props.caption {
                <span class="thumbnail-caption"><span>{caption}</span></span>
            } else if let ThumbnailCaption::Html(caption) = &props.caption {
                <span class="thumbnail-caption">{caption.clone()}</span>
            }
        </>
    };
    match props.container_type {
        ContainerType::div => html! {
            <div class="thumbnail-container clickable" {onclick}>{content}</div>
        },
        ContainerType::td => html! {
            <td class="thumbnail-container clickable" {onclick}>{content}</td>
        },
    }
}
