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

use gloo_console::{error, log};
use reqwest::Url;
use yew::prelude::*;
use yew_hooks::{use_async_with_options, UseAsyncHandle, UseAsyncOptions};

use crate::components::modals::{thumbnail::ThumbnailModal, ModalMessage};
use crate::hooks::use_async_suspension;
use crate::{ModalRendererControls, SettingsContext};
use crate::utils::RcEq;

use super::common::{ThumbgenStats, ThumbnailKey};
use super::local::{LocalBlobLink, LocalThumbGenerator};
use super::remote::{Error, RemoteBlobLink, ThumbnailWorker};
use super::worker_api::{ThumbnailWorkerRequest, WorkerSetting};

#[derive(Clone, Eq, PartialEq)]
pub enum Thumbgen {
    Remote(ThumbnailWorker),
    Local{
        gen: LocalThumbGenerator,
        /// Error from the attempt to initialize the remote thumbnail worker
        error: RcEq<super::remote::Error>,
    },
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
    pub async fn get_thumbnail(&self, key: &ThumbnailKey) -> Result<ThumbnailUrl, Error> {
        match self {
            Self::Remote(worker) => worker.get_thumbnail(key.clone()).await.map(|t| ThumbnailUrl::Remote(Rc::new(t))),
            Self::Local { gen, .. } => gen.get_thumb(key).await.map(ThumbnailUrl::Local).map_err(|e| Error::Remote(e.into())),
        }
    }

    pub async fn get_stats(&self) -> Result<ThumbgenStats, Error> {
        match self {
            Self::Remote(worker) => worker.get_stats().await,
            Self::Local { gen, .. } => Ok(ThumbgenStats { cache_stats: gen.get_stats(), worker_stats: None }),
        }
    }

    pub async fn clear_errors(&self) {
        match self {
            Self::Remote(worker) => drop(worker.request(ThumbnailWorkerRequest::ClearErrors).await),
            Self::Local { gen, .. } => drop(gen.clear_errors()),
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
                if gen.is_protocol_mismatched() {
                    "Ready (mismatched protocol)"
                } else {
                    "Ready"
                }
            }
        }
    }
}


#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ThumbgenRefreshValue(pub u8);

impl ThumbgenRefreshValue {
    fn bump(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

pub type ThumbgenRefreshContext = UseStateHandle<ThumbgenRefreshValue>;
pub trait TRExt {
    fn trigger_refresh(&self);
}

impl TRExt for ThumbgenRefreshContext {
    fn trigger_refresh(&self) {
        self.set(self.bump());
    }
}

#[derive(Properties, PartialEq)]
pub struct ThumbnailGeneratorProviderProps {
    pub children: Html
}

#[function_component]
pub fn ThumbgenProvider(props: &ThumbnailGeneratorProviderProps) -> Html {
    let settings_context: SettingsContext = use_context().expect("ThumbnailProvider must be placed under a SettingsContext provider");
    let settings = settings_context.settings();
    let disable_sharedworker = settings.disable_sharedworker;
    let thumgen_state: UseAsyncHandle<Thumbgen, ()> = use_async_with_options(async move {
        if disable_sharedworker {
            Ok(Thumbgen::Local { 
                gen: LocalThumbGenerator::new(), 
                error: RcEq::new(Error::ConfigDisabled), 
            })
        } else {
            Ok(match ThumbnailWorker::new().await {
                Ok(worker) => Thumbgen::Remote(worker),
                Err(err) => Thumbgen::Local {
                    gen: LocalThumbGenerator::new(),
                    error: RcEq::new(err),
                },
            })
        }
    }, UseAsyncOptions::enable_auto());
    let refresh_state = use_state(|| ThumbgenRefreshValue(0));

    // Thumbgen API URL updates
    use_memo((thumgen_state.data.clone(), settings.thumbgen_api_base_url.clone()), |(thumbgen, api_base_url)| {
        match thumbgen {
            None => (),
            Some(Thumbgen::Remote(worker)) => {
                if let Err(e) = worker.post_request(ThumbnailWorkerRequest::SettingUpdated {
                    setting: WorkerSetting::ThumbgenBaseUrl(api_base_url.to_string())
                }) {
                    error!(format!("Failed to notify Thumbgen::Remote about a thumbgen API base URL change: {e}"));
                }
            },
            Some(Thumbgen::Local { r#gen, .. }) => {
                let mut url = match Url::parse(api_base_url) {
                    Ok(url) => url,
                    Err(e) => return error!(format!("Failed to parse new ThumbgenBaseUrl: {e}")),
                };
                {
                    let Ok(mut path) = url.path_segments_mut() else {
                        return error!(format!("Failed to append API endpoint to new ThumbgenBaseUrl: {api_base_url} cannot be a base"))
                    };
                    path.extend(&["api", "v1", "getThumbnail"]);
                };
                r#gen.set_api_url(url);
                let errors_removed = r#gen.clear_errors();
                log!(format!("Cleared {errors_removed} error entries after updating thumbgen API URL"));
            }
        }
        refresh_state.trigger_refresh();
    });

    html! {
        <ContextProvider<ThumbgenContext> context={thumgen_state.data.clone()}>
        <ContextProvider<ThumbgenRefreshContext> context={refresh_state.clone()}>
            { props.children.clone() }
        </ContextProvider<ThumbgenRefreshContext>>
        </ContextProvider<ThumbgenContext>>
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
