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
// NOTE: This file is compiled into the worker wasm binary!

use std::{cell::{Cell, OnceCell, RefCell}, collections::HashMap, rc::Rc};

use common::{ThumbgenStats, WorkerStats};
use gloo_console::{error, log, warn};
use slab::Slab;
use wasm_bindgen::{closure::Closure, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{js_sys::{self, Array, Reflect, Uint8Array}, MessageEvent, MessagePort, SharedWorkerGlobalScope};

pub mod local;
mod common;
#[allow(dead_code)]
mod utils;
pub mod worker_api;

use local::{LocalBlobLink, LocalThumbGenerator};
use worker_api::{RawRemoteRef, ThumbnailWorkerRequest, ThumbnailWorkerRequestMessage, ThumbnailWorkerResponse, ThumbnailWorkerResponseMessage};

const PING_CHECK_INTERVAL: i32 = 30_000;
const MAX_PING_FAILS: u8 = 3;

struct WorkerContext {
    global: SharedWorkerGlobalScope,
    clients: RefCell<Slab<Rc<RemoteClient>>>,
    thumbgen: LocalThumbGenerator,
    connect_event_closure: OnceCell<Closure<dyn FnMut(MessageEvent)>>,
    message_event_closure: OnceCell<Closure<dyn FnMut(MessageEvent)>>,
    message_error_event_closure: OnceCell<Closure<dyn FnMut(MessageEvent)>>,
    client_sweep_closure: OnceCell<Closure<dyn FnMut()>>,
}

struct RemoteClient {
    port: MessagePort,
    ping_received: Cell<bool>,
    checks_since_last_ping: Cell<u8>,
    next_ref_id: Cell<u16>,
    held_refs: RefCell<HashMap<u16, Rc<LocalBlobLink>>>,
}

impl RemoteClient {
    fn new(port: MessagePort) -> Self {
        RemoteClient { 
            port, 
            ping_received: Cell::new(true),
            checks_since_last_ping: Cell::new(0),
            next_ref_id: Cell::new(0), 
            held_refs: RefCell::new(HashMap::new()),
        }
    }

    fn store_ref(&self, r#ref: Rc<LocalBlobLink>) -> u16 {
        let mut held_refs = self.held_refs.borrow_mut();
        assert!(held_refs.len() < u16::MAX.into(), "Out of ref IDs!");
        let ref_id = loop {
            let new_id = self.next_ref_id.replace(self.next_ref_id.get().wrapping_add(1));
            
            if !held_refs.contains_key(&new_id) {
                break new_id;
            }
        };
        assert!(held_refs.insert(ref_id, r#ref).is_none(), "Replaced a ref by mistake!");
        ref_id
    }

    fn drop_ref(&self, ref_id: u16) {
        if self.held_refs.borrow_mut().remove(&ref_id).is_none() {
            warn!("Attempted to drop a non-existent ref!");
        }
    }
}

impl Drop for RemoteClient {
    fn drop(&mut self) {
        let remaining_refs = self.held_refs.get_mut().len();
        if remaining_refs > 0 {
            warn!(format!("RemoteClient dropped with {remaining_refs} held refs"));
        }    }
}

impl WorkerContext {
    fn new() -> &'static Self {
        Box::leak(Box::new(WorkerContext {
            global: js_sys::global().unchecked_into(),
            clients: RefCell::new(Slab::with_capacity(8)),
            thumbgen: LocalThumbGenerator::new(),
            connect_event_closure: OnceCell::new(),
            message_event_closure: OnceCell::new(),
            message_error_event_closure: OnceCell::new(),
            client_sweep_closure: OnceCell::new(),
        }))
    }

    fn get_connect_event_closure(&'static self) -> &Closure<dyn FnMut(MessageEvent)> {
        self.connect_event_closure.get_or_init(|| Closure::new(|e| self.handle_connect_event(e)))
    }

    fn get_message_event_closure(&'static self) -> &Closure<dyn FnMut(MessageEvent)> {
        self.message_event_closure.get_or_init(|| Closure::new(|e| self.handle_message_event(e)))
    }

    fn get_message_error_event_closure(&self) -> &Closure<dyn FnMut(MessageEvent)> {
        self.message_error_event_closure.get_or_init(|| Closure::new(Self::handle_message_error_event))
    }

    fn get_client_sweep_closure(&'static self) -> &Closure<dyn FnMut()> {
        self.client_sweep_closure.get_or_init(|| Closure::new(|| self.client_sweep()))
    }

    fn client_sweep(&self) {
        let mut count_timing_out = 0;
        let mut clients = self.clients.borrow_mut();
        for (_, client) in clients.iter_mut() {
            if client.ping_received.get() {
                client.checks_since_last_ping.set(0);
            } else {
                client.checks_since_last_ping.set(client.checks_since_last_ping.get() + 1);
                count_timing_out += 1;
            }
            client.ping_received.set(false);
        }
        if count_timing_out > 0 {
            warn!(format!("Client sweep: {count_timing_out} clients are timing out"));
        }

        let before_clear = clients.len();
        clients.retain(|_, client| client.checks_since_last_ping.get() < MAX_PING_FAILS);
        let count_timed_out = before_clear - clients.len();
        if count_timed_out > 0 {
            warn!(format!("Client sweep: {count_timed_out} clients have timed out"));
        }
    }

    fn handle_connect_event(&'static self, event: MessageEvent) {
        let Ok(port) = event.ports().get(0).dyn_into::<MessagePort>() else {
            error!("Connect event did not contain a MessagePort!", event);
            return;
        };
        
        port.set_onmessage(Some(self.get_message_event_closure().as_ref().unchecked_ref()));
        port.set_onmessageerror(Some(self.get_message_error_event_closure().as_ref().unchecked_ref()));
        self.register_client(port);
    }

    fn handle_message_event(&'static self, event: MessageEvent) {
        let data: Uint8Array = match event.data().dyn_into() {
            Ok(data) => data,
            Err(data) => {
                error!("Got a message that wasn't a Uint8Array!", event, data);
                return;
            }
        };
        let port: MessagePort = event.target().expect("target should be set").unchecked_into();

        let data = data.to_vec();
        let message: ThumbnailWorkerRequestMessage = match bincode::deserialize(&data) {
            Ok(msg) => msg,
            Err(err) => {
                error!(format!("Failed to deserialize a message: {err:?}"));
                // this will be missing data, but at least the caller should get notified of the error
                port.reply(ThumbnailWorkerResponseMessage {
                    id: 0,
                    response: ThumbnailWorkerResponse::DeserializationError { received_data: data }
                });
                return;
            }
        };

        spawn_local(self.process_message(port, message));
    }

    fn handle_message_error_event(event: MessageEvent) {
        error!("Got a message error event!", event);
    }

    fn get_event_backlog(&self) -> Option<Array> {
        match Reflect::get(&self.global, &"events".into()) {
            Err(err) => {
                error!("Failed to retrieve the event backlog array from JS", err);
                None
            },
            Ok(arr) => {
                if arr.is_null() || arr.is_undefined() {
                    error!("Event backlog array is null or undefined!");
                    return None
                }
                match arr.dyn_into() {
                    Ok(arr) => Some(arr),
                    Err(arr) => {
                        error!("Event backlog array is not an array!", arr);
                        None
                    },
                }
            },
        }
    }

    fn register_client(&self, port: MessagePort) {
        let mut clients = self.clients.borrow_mut();
        if clients.iter().any(|(_, client)| client.port == port) {
            warn!("Attempted to re-register an existing client!");
        } else {
            clients.insert(Rc::new(RemoteClient::new(port)));
            log!("Client registered");
        }
    }

    fn get_client(&self, port: &MessagePort) -> Rc<RemoteClient> {
        let mut clients = self.clients.borrow_mut();
        if let Some((_, client)) = clients.iter().find(|(_, client)| client.port == *port) {
            return client.clone();
        }

        warn!("A client was missing from the clients list!");
        let client = Rc::new(RemoteClient::new(port.clone()));
        clients.insert(client.clone());
        client
    }

    async fn process_message(&'static self, port: MessagePort, message: ThumbnailWorkerRequestMessage) {
        // Get client, mark it as alive
        let client = self.get_client(&port);
        client.ping_received.set(true);
        client.checks_since_last_ping.set(0);

        // Actually process the message
        let ThumbnailWorkerRequestMessage { id, request } = message;
        match request {
            ThumbnailWorkerRequest::Version { version, git_hash, git_dirty } => {
                if version != built_info::PKG_VERSION || git_hash.as_deref() != built_info::GIT_COMMIT_HASH || git_dirty != built_info::GIT_DIRTY {
                    warn!(format!("Version mismatch detected! Message (de)serialization errors may occur!\nNew client's version: {version}, git hash: {git_hash:?}, git dirty: {git_dirty:?}\nWorker version: {}, git hash: {:?}, git dirty: {:?}\nClose all DeArrow Browser windows to resolve this issue.", built_info::PKG_VERSION, built_info::GIT_COMMIT_HASH, built_info::GIT_DIRTY));
                };
                
                port.reply(ThumbnailWorkerResponseMessage {
                    id,
                    response: ThumbnailWorkerResponse::Version { version: built_info::PKG_VERSION.to_owned(), git_hash: built_info::GIT_COMMIT_HASH.map(ToOwned::to_owned), git_dirty: built_info::GIT_DIRTY },
                });
            },
            ThumbnailWorkerRequest::BlobLinkDropped { ref_id } => {
                client.drop_ref(ref_id);
            },
            ThumbnailWorkerRequest::GetThumbnail { key } => {
                let result = self.thumbgen.get_thumb(&key).await
                    .map(|thumb_url| {
                        RawRemoteRef {
                            url: thumb_url.inner_url().into(),
                            ref_id: client.store_ref(thumb_url),
                        }
                    })
                    .map_err(Into::into);
                port.reply(ThumbnailWorkerResponseMessage { 
                    id, 
                    response: ThumbnailWorkerResponse::Thumbnail { r#ref: result }
                });
            },
            ThumbnailWorkerRequest::GetStats => {
                port.reply(ThumbnailWorkerResponseMessage { id, response: ThumbnailWorkerResponse::Stats { stats: ThumbgenStats { 
                    cache_stats: self.thumbgen.get_stats(),
                    worker_stats: Some(WorkerStats { 
                        clients: self.clients.borrow().len(),
                        this_client_refs: client.held_refs.borrow().len(),
                    }),
                }}});
            },
            ThumbnailWorkerRequest::Ping => {
                port.reply(ThumbnailWorkerResponseMessage { id, response: ThumbnailWorkerResponse::Pong });
            },
            ThumbnailWorkerRequest::Disconnecting => {
                // Find and drop the client
                let mut clients = self.clients.borrow_mut();
                if let Some((key, _)) = clients.iter().find(|(_, v)| Rc::ptr_eq(v, &client)) {
                    clients.remove(key);
                    log!("Client disconnected");
                } else {
                    error!("Failed to find & remove a client that sent a Disconnecting request!");
                }
            }
        };
    }
}

trait MessagePortExt {
    fn reply(&self, message: ThumbnailWorkerResponseMessage);
}

impl MessagePortExt for MessagePort {
    fn reply(&self, message: ThumbnailWorkerResponseMessage) {
        let message = match bincode::serialize(&message) {
            Ok(m) => m,
            Err(err) => {
                error!(format!("Failed to serialize reply: {err:?}"));
                return;
            }
        };
        let message: Uint8Array = (&*message).into();
        if let Err(err) = self.post_message_with_transferable(&message, &Array::of1(&message.buffer())) {
            error!("Failed to send reply:", err);
        }
    }
}

fn main() {
    log!("Wasm loaded!");
    // Create global context object
    let ctx: &'static WorkerContext = WorkerContext::new();

    // Register client sweep interval
    if let Err(err) = ctx.global.set_interval_with_callback_and_timeout_and_arguments_0(ctx.get_client_sweep_closure().as_ref().unchecked_ref(), PING_CHECK_INTERVAL) {
        error!("Failed to set interval for thumbgen cache sweeps!", err);
    }

    // Register connection callback
    ctx.global.set_onconnect(Some(ctx.get_connect_event_closure().as_ref().unchecked_ref()));

    // Process event backlog
    log!("Callbacks set, processing event backlog");
    if let Some(backlog) = ctx.get_event_backlog() {
        for event in backlog {
            match event.dyn_into() {
                Ok(event) => ctx.handle_connect_event(event),
                Err(event) => error!("Non-MessageEvent item found in event backlog: ", event),
            }
        }
    }
    log!("Backlog processed!");
}

pub mod built_info {
    // Contents generated by buildscript, using built
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
