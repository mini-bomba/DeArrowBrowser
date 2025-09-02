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

use std::{any::type_name, marker::PhantomData, rc::Rc};

use cloneable_errors::{ErrorContext, ResContext};
use gloo_console::warn;
use reqwest::Url;
use serde::de::DeserializeOwned;
use yew::{
    html, html::Scope, platform::spawn_local, Callback, Component, Context, ContextHandle, Html,
    Properties,
};

use crate::{
    components::tables::{
        renderer::{PaginatedTableRenderer, TableRenderer}, switch::Tabs, r#trait::TableRender
    },
    constants::REQWEST_CLIENT,
    contexts::WindowContext,
    utils_app::{CancelHandle, CancelWatcher, RcEq}, utils_common::ReqwestResponseExt
};

pub trait Endpoint: Sized + Eq + 'static {
    type Item: TableRender;
    type LoadProgress: DeserializeOwned;

    fn create_url(&self, base_url: &Url) -> Url;
    fn render_load_progress(&self, _progress: &Self::LoadProgress) -> Html {
        html! {<b>{"Yep, still loading..."}</b>}
    }
}

#[derive(Properties)]
pub struct RemoteTableProps<E: Endpoint> {
    pub endpoint: E,
    #[prop_or_default]
    pub settings: <E::Item as TableRender>::Settings,
    #[prop_or_default]
    pub item_count_update: Callback<Option<usize>>,
}

impl<E: Endpoint> PartialEq for RemoteTableProps<E> {
    fn eq(&self, other: &Self) -> bool {
        self.settings == other.settings && self.endpoint == other.endpoint
    }
}

impl<E: Endpoint> Eq for RemoteTableProps<E> {}

pub enum LoadProgress<E: Endpoint> {
    LoadingInitial,
    LoadingProgress(E::LoadProgress),
    Ready(RcEq<[E::Item]>),
    Failed(ErrorContext),
}

impl<E: Endpoint> LoadProgress<E> {
    fn len(&self) -> Option<usize> {
        match self {
            Self::Ready(items) => Some(items.len()),
            _ => None,
        }
    }
}

pub struct RemotePaginatedTable<E: Endpoint, S: Tabs> {
    origin: Url,

    data: LoadProgress<E>,
    handle: CancelHandle,

    _wc_listener: ContextHandle<Rc<WindowContext>>,
    _phantom: PhantomData<S>,
}

pub enum RemoteTableMessage<E: Endpoint> {
    OriginUpdated(Url),
    DetailsFetched {
        watcher: CancelWatcher,
        data: LoadProgress<E>,
    },
}

fn fetch_data<E: Endpoint, C: Component<Message = RemoteTableMessage<E>>>(
    scope: Scope<C>,
    watcher: CancelWatcher,
    url: Url,
) {
    spawn_local(async move {
        let data = async {
            loop {
                if watcher.check() {
                    return Ok(None);
                }
                let resp = REQWEST_CLIENT
                    .get(url.clone())
                    .header("Accept", "application/json")
                    .send()
                    .await
                    .context("API request failed")?;
                if resp.status().as_u16() == 333 {
                    let res: Result<(), ErrorContext> = async {
                        let progress = resp.json::<E::LoadProgress>().await.context("Failed to deserialize the 333 response")?;
                        scope.send_message(RemoteTableMessage::DetailsFetched { watcher: watcher.clone(), data: LoadProgress::LoadingProgress(progress) });
                        Ok(())
                    }.await;
                    if let Err(err) = res {
                        warn!(format!("Got a 333 response for endpoint {}, but failed to deserialize it: {err:?}", type_name::<E>()));
                    }
                    continue;
                }
                if watcher.check() {
                    return Ok(None);
                }
                return resp.check_status()
                    .await?
                    .json::<Rc<[E::Item]>>()
                    .await
                    .context("Failed to deserialize API response")
                    .map(RcEq)
                    .map(Some)
            }
        }
        .await;
        match data {
            Ok(None) => (),
            Err(err) => scope.send_message(RemoteTableMessage::DetailsFetched {
                watcher,
                data: LoadProgress::Failed(err),
            }),
            Ok(Some(data)) => scope.send_message(RemoteTableMessage::DetailsFetched {
                watcher,
                data: LoadProgress::Ready(data),
            }),
        }
    });
}

impl<E: Endpoint, S: Tabs> RemotePaginatedTable<E, S> {
    fn refresh(&mut self, ctx: &Context<Self>) {
        self.data = LoadProgress::LoadingInitial;
        self.handle = CancelHandle::new();
        let watcher = self.handle.watch();
        let props = ctx.props();
        let url = props.endpoint.create_url(&self.origin);
        let scope = ctx.link().clone();
        props.item_count_update.emit(None);
        fetch_data::<E, Self>(scope, watcher, url);
    }
}

impl<E: Endpoint, S: Tabs> Component for RemotePaginatedTable<E, S> {
    type Properties = RemoteTableProps<E>;
    type Message = RemoteTableMessage<E>;

    fn create(ctx: &yew::Context<Self>) -> Self {
        let scope = ctx.link();

        let (wc, wc_listener) = scope
            .context(scope.callback(|wc: Rc<WindowContext>| {
                RemoteTableMessage::OriginUpdated(wc.origin.clone())
            }))
            .expect("WindowContext should be available");

        let mut this = Self {
            origin: wc.origin.clone(),

            data: LoadProgress::LoadingInitial,
            handle: CancelHandle::new(),

            _wc_listener: wc_listener,
            _phantom: PhantomData,
        };
        this.refresh(ctx);
        this
    }

    fn view(&self, ctx: &yew::Context<Self>) -> yew::Html {
        let props = ctx.props();
        match &self.data {
            LoadProgress::LoadingInitial => html! {<center><b>{"Loading..."}</b></center>},
            LoadProgress::LoadingProgress(progress) => {
                html! {<center>{props.endpoint.render_load_progress(progress)}</center>}
            }
            LoadProgress::Failed(e) => html! {
                <center>
                    <b>{"Failed to fetch details from the API :/"}</b>
                    <pre>{format!("{e:?}")}</pre>
                </center>
            },
            LoadProgress::Ready(items) => html! {
                <PaginatedTableRenderer<E::Item, S> {items} settings={props.settings} />
            },
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            RemoteTableMessage::OriginUpdated(origin) => {
                if self.origin == origin {
                    false
                } else {
                    self.origin = origin;
                    self.refresh(ctx);
                    true
                }
            }
            RemoteTableMessage::DetailsFetched { watcher, data } => {
                if self.handle.compare(&watcher) {
                    ctx.props().item_count_update.emit(data.len());
                    self.data = data;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if old_props.endpoint == ctx.props().endpoint {
            false
        } else {
            self.refresh(ctx);
            true
        }
    }
}

pub struct RemoteUnpaginatedTable<E: Endpoint> {
    origin: Url,

    data: LoadProgress<E>,
    handle: CancelHandle,

    _wc_listener: ContextHandle<Rc<WindowContext>>,
}

impl<E: Endpoint> RemoteUnpaginatedTable<E> {
    fn refresh(&mut self, ctx: &Context<Self>) {
        self.data = LoadProgress::LoadingInitial;
        self.handle = CancelHandle::new();
        let watcher = self.handle.watch();
        let props = ctx.props();
        let url = props.endpoint.create_url(&self.origin);
        let scope = ctx.link().clone();
        props.item_count_update.emit(None);
        fetch_data::<E, Self>(scope, watcher, url);
    }
}

impl<E: Endpoint> Component for RemoteUnpaginatedTable<E> {
    type Properties = RemoteTableProps<E>;
    type Message = RemoteTableMessage<E>;

    fn create(ctx: &yew::Context<Self>) -> Self {
        let scope = ctx.link();

        let (wc, wc_listener) = scope
            .context(scope.callback(|wc: Rc<WindowContext>| {
                RemoteTableMessage::OriginUpdated(wc.origin.clone())
            }))
            .expect("WindowContext should be available");

        let mut this = Self {
            origin: wc.origin.clone(),

            data: LoadProgress::LoadingInitial,
            handle: CancelHandle::new(),

            _wc_listener: wc_listener,
        };
        this.refresh(ctx);
        this
    }

    fn view(&self, ctx: &yew::Context<Self>) -> yew::Html {
        let props = ctx.props();
        match &self.data {
            LoadProgress::LoadingInitial => html! {<center><b>{"Loading..."}</b></center>},
            LoadProgress::LoadingProgress(progress) => {
                html! {<center>{props.endpoint.render_load_progress(progress)}</center>}
            }
            LoadProgress::Failed(e) => html! {
                <center>
                    <b>{"Failed to fetch details from the API :/"}</b>
                    <pre>{format!("{e:?}")}</pre>
                </center>
            },
            LoadProgress::Ready(items) => html! {
                <TableRenderer<E::Item> {items} settings={props.settings} />
            },
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            RemoteTableMessage::OriginUpdated(origin) => {
                if self.origin == origin {
                    false
                } else {
                    self.origin = origin;
                    self.refresh(ctx);
                    true
                }
            }
            RemoteTableMessage::DetailsFetched { watcher, data } => {
                if self.handle.compare(&watcher) {
                    ctx.props().item_count_update.emit(data.len());
                    self.data = data;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if old_props.endpoint == ctx.props().endpoint {
            false
        } else {
            self.refresh(ctx);
            true
        }
    }
}
