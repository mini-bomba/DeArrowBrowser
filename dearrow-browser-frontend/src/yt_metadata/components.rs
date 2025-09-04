/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2025 mini_bomba
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

use std::{marker::PhantomData, rc::Rc};

use yew::{
    html,
    html::{ChildrenProps, ImplicitClone},
    platform::spawn_local,
    AttrValue, Component, Context, ContextHandle, ContextProvider, Html, Properties,
};
use yew_router::prelude::Link;

use crate::{
    components::icon::{Icon, IconType},
    pages::MainRoute,
    utils_app::SimpleLoadState,
    worker_client::{Error, WorkerState},
    yt_metadata::{common::VideoMetadata, local::LocalMetadataCache, remote::RemoteMetadataCache},
};

#[derive(Clone, PartialEq, Eq)]
pub enum MetadataCache {
    Local(LocalMetadataCache),
    Remote(RemoteMetadataCache),
}

impl ImplicitClone for MetadataCache {}

impl MetadataCache {
    fn from_worker_state(worker_state: WorkerState) -> Option<Self> {
        match worker_state {
            WorkerState::Loading => None,
            WorkerState::Ready(client) => Some(Self::Remote(RemoteMetadataCache { client })),
            WorkerState::Failed(..) => Some(Self::Local(LocalMetadataCache::new())),
        }
    }

    pub async fn get_metadata(&self, video_id: Rc<str>) -> Result<VideoMetadata, Error> {
        match self {
            Self::Local(cache) => cache.get_metadata(video_id).await.map_err(Error::ErrorContext),
            Self::Remote(cache) => cache.get_metadata(video_id).await,
        }
    }

    pub fn clear_errors(&self) {
        match self {
            Self::Local(cache) => drop(cache.clear_errors()),
            Self::Remote(cache) => cache.clear_errors(),
        }
    }

    pub fn clear_cache(&self) {
        match self {
            Self::Local(cache) => drop(cache.clear_cache()),
            Self::Remote(cache) => cache.clear_cache(),
        }
    }
}

pub type MetadataCacheContext = Option<MetadataCache>;

pub struct MetadataCacheProvider {
    cache: MetadataCacheContext,

    _worker_handle: ContextHandle<WorkerState>,
}

impl Component for MetadataCacheProvider {
    type Message = WorkerState;
    type Properties = ChildrenProps;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();
        let (worker, worker_handle) = scope.context(scope.callback(|x| x)).expect("Worker should be available");

        Self {
            cache: MetadataCache::from_worker_state(worker),

            _worker_handle: worker_handle,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <ContextProvider<MetadataCacheContext> context={&self.cache}>
                {ctx.props().children.clone()}
            </ContextProvider<MetadataCacheContext>>
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        self.cache = MetadataCache::from_worker_state(msg);
        true
    }
}

pub trait RenderMetadata: Sized + 'static {
    fn render(metadata: &SimpleLoadState<VideoMetadata>) -> Html;
}

pub struct VideoMetadataRenderer<R: RenderMetadata> {
    cache: MetadataCacheContext,

    version: u8,
    metadata: SimpleLoadState<VideoMetadata>,

    _cache_handle: ContextHandle<MetadataCacheContext>,
    _phantom: PhantomData<R>,
}
pub type OriginalTitle = VideoMetadataRenderer<RenderTitle>;
pub type ChannelLink = VideoMetadataRenderer<RenderChannelLink>;

impl<R: RenderMetadata> VideoMetadataRenderer<R> {
    fn refresh(&mut self, ctx: &Context<Self>) {
        let Some(cache) = self.cache.clone() else { return };

        let version = self.version.wrapping_add(1);
        self.version = version;
        self.metadata = SimpleLoadState::Loading;

        let scope = ctx.link().clone();
        let video_id = ctx.props().video_id.clone();

        spawn_local(async move {
            scope.send_message(VideoMetadataRendererMsg::MetadataFetched { 
                data: cache.get_metadata(video_id).await,
                version,
            });
        });
    }
}

#[derive(Properties, PartialEq, Eq)]
pub struct VideoIdParam {
    pub video_id: Rc<str>,
}

pub enum VideoMetadataRendererMsg {
    MetadataCacheUpdate(MetadataCacheContext),
    MetadataFetched {
        data: Result<VideoMetadata, Error>,
        version: u8,
    },
}

impl<R: RenderMetadata> Component for VideoMetadataRenderer<R> {
    type Properties = VideoIdParam;
    type Message = VideoMetadataRendererMsg;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();

        let (cache, cache_handle) = scope.context(scope.callback(VideoMetadataRendererMsg::MetadataCacheUpdate)).expect("Metadata cache should be available");

        let mut this = Self {
            cache,

            version: 0,
            metadata: SimpleLoadState::Loading,

            _cache_handle: cache_handle,
            _phantom: PhantomData,
        };
        this.refresh(ctx);
        this
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        R::render(&self.metadata)
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            VideoMetadataRendererMsg::MetadataCacheUpdate(cache) => {
                let should_refresh = self.cache.is_none();
                self.cache = cache;
                if should_refresh {
                    self.refresh(ctx);
                }
                false
            },
            VideoMetadataRendererMsg::MetadataFetched { data, version } => {
                if version != self.version {
                    return false;
                }
                if let Err(err) = &data {
                    err.log(&format!("Failed to fetch metadata for video {}", &ctx.props().video_id));
                }
                self.metadata = data.ok().into();
                true
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if ctx.props().video_id == old_props.video_id {
            return false;
        }
        self.refresh(ctx);
        true
    }
}

pub struct RenderTitle;

impl RenderMetadata for RenderTitle {
    fn render(metadata: &SimpleLoadState<VideoMetadata>) -> Html {
        match metadata {
            SimpleLoadState::Loading => html! {<em>{"Loading..."}</em>},
            SimpleLoadState::Failed(()) => html! {<em>{"Unknown"}</em>},
            SimpleLoadState::Ready(m) => html! {<span>{&m.title}</span>}
        }
    }
}

pub struct RenderChannelLink;

impl RenderMetadata for RenderChannelLink {
    fn render(metadata: &SimpleLoadState<VideoMetadata>) -> Html {
        match metadata {
            SimpleLoadState::Loading => html! {<em>{"Loading..."}</em>},
            SimpleLoadState::Failed(()) => html! {<em>{"Unknown"}</em>},
            SimpleLoadState::Ready(m) => {
                let route = MainRoute::Channel {
                    id: AttrValue::Rc(m.channel.clone()),
                };
                html! {
                    <Link<MainRoute> to={route}>{&m.channel}<Icon r#type={IconType::DABLogo} /></Link<MainRoute>>
                }
            }
        }
    }
}
