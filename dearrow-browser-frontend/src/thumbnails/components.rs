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

use yew::prelude::*;
use yew_hooks::{use_async_with_options, UseAsyncHandle, UseAsyncOptions};

use crate::{hooks::use_async_suspension, utils::RcEq};

use super::{common::{ThumbgenStats, ThumbnailKey}, local::{LocalBlobLink, LocalThumbGenerator}, remote::{Error, RemoteBlobLink, ThumbnailWorker}};

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
}

pub type ThumbgenContext = Option<Thumbgen>;

#[derive(Properties, PartialEq)]
pub struct ThumbnailGeneratorProviderProps {
    pub children: Html
}

#[function_component]
pub fn ThumbgenProvider(props: &ThumbnailGeneratorProviderProps) -> Html {
    let state: UseAsyncHandle<Thumbgen, ()> = use_async_with_options(async move {
        Ok(match ThumbnailWorker::new().await {
            Ok(worker) => Thumbgen::Remote(worker),
            Err(err) => Thumbgen::Local {
                gen: LocalThumbGenerator::new(),
                error: RcEq::new(err),
            },
        })
    }, UseAsyncOptions::enable_auto());

    html! {
        <ContextProvider<ThumbgenContext> context={state.data.clone()}>
            { props.children.clone() }
        </ContextProvider<ThumbgenContext>>
    }
}

#[derive(Properties, PartialEq)]
pub struct ThumbnailProps {
    pub thumb_key: ThumbnailKey,
}

#[function_component]
pub fn Thumbnail(props: &ThumbnailProps) -> HtmlResult {
    let generator: ThumbgenContext = use_context().expect("Thumbnail must be run under a ThumbnailGeneratorProvider");
    let thumbnail = use_async_suspension(|(generator, key)| async move {
        Some(generator?.get_thumbnail(&key).await)
    }, (generator, props.thumb_key.clone()))?;
    
    Ok(match *thumbnail {
        None => html! { <span class="thumbnail-error">{"Waiting for thumbnail generator..."}</span>},
        Some(Err(ref err)) => html! { <span class="thumbnail-error">{format!("{err:?}")}</span> },
        Some(Ok(ref url)) => html! { <img class="thumbnail" src={Rc::from(url.get_url())} /> },
    })
}
