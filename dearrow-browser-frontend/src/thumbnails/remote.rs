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

use std::{cell::{Cell, OnceCell, RefCell}, collections::HashMap, fmt::{Debug, Display}, rc::Rc};

use error_handling::ResContext;
use gloo_console::{error, warn};
use futures::{channel::oneshot::*, select_biased, future::FutureExt};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{js_sys::{Array, Object, Uint8Array}, window, MessageEvent, PageTransitionEvent, SharedWorker, WorkerOptions};

use crate::{built_info, utils::RcEq};

use super::common::{ThumbgenStats, ThumbnailKey};
use super::utils::{make_jsstring, sleep, EventCellsExt, EventListener, Interval};
use super::worker_api::*;

const WORKER_INIT_TIMEOUT: u32 = 30_000; // 0.5 min
const WORKER_KEEPALIVE_INTERVAL: i32 = 20_000;

/// Represents a shared reference to the underlying object URL (aka bloblink)
///
/// The object URL is owned by a remote ``ThumbnailWorker``.
///
/// Uncloneable, dropping will notify the remote worker which may revoke the URL
pub struct RemoteBlobLink {
    worker: ThumbnailWorker,
    url: Box<str>,
    ref_id: u16,
}

impl Drop for RemoteBlobLink {
    fn drop(&mut self) {
        if let Err(err) = self.worker.post_request(ThumbnailWorkerRequest::BlobLinkDropped { ref_id: self.ref_id }) {
            err.log("Failed to notify worker about a RemoteBlobLink being dropped");
        };
    }
}

impl Debug for RemoteBlobLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteBlobLink").field("url", &self.url).field("ref_id", &self.ref_id).finish_non_exhaustive()
    }
}

impl PartialEq for RemoteBlobLink {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}
impl Eq for RemoteBlobLink {}

impl RemoteBlobLink {
    /// Borrows the inner URL string.
    /// As the ``str`` representing the URL is cloneable, this function is marked unsafe.
    ///
    /// # Resource Lifetime
    /// ``RemoteBlobLink`` must outlive any clones of the inner URL - the URL may be invalidated when this
    /// struct is dropped.
    pub fn inner_url(&self) -> &str {
        &self.url
    }
}

#[derive(Clone)]
pub struct ThumbnailWorker {
    inner: Rc<InnerTW>,
}

impl PartialEq for ThumbnailWorker {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
impl Eq for ThumbnailWorker {}

struct InnerTW {
    worker: SharedWorker,
    message_handler: OnceCell<Closure<dyn FnMut(MessageEvent)>>,
    message_queue: RefCell<HashMap<u16, Sender<Result<ThumbnailWorkerResponse, Error>>>>,
    next_id: Cell<u16>,
    keepalive_interval: OnceCell<Interval<dyn FnMut()>>,
    pagehide_listener: OnceCell<EventListener<dyn FnMut(PageTransitionEvent)>>,
    protocol_mismatch: Cell<bool>,
}

#[derive(PartialEq)]
pub enum Error {
    Bincode(RcEq<bincode::Error>),
    JS(JsValue),
    Cancelled(Canceled),
    /// worker couldn't deserialize the request
    ProtocolError,
    /// timed out waiting for the worker to respond
    WorkerInitializationTimeout,
    /// shared worker impl disabled via settings
    ConfigDisabled,
    /// error from the thumbnail worker
    Remote(RemoteThumbnailGenerationError),
}

impl Error {
    pub fn log(&self, context: &str) {
        match self {
            Self::JS(err) => error!(context, err),
            other => error!(context, format!("{other:?}")),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bincode(RcEq(err)) => write!(f, "Message serialization failed: {err:?}"),
            Self::Cancelled(err) => write!(f, "Waiting for response got cancelled: {err:?}"),
            Self::ProtocolError => write!(f, "Window-worker protocol mismatch"),
            Self::WorkerInitializationTimeout => write!(f, "Timed out waiting for the worker to initialize"),
            Self::ConfigDisabled => write!(f, "SW impl disabled in settings"),
            Self::Remote(err) => write!(f, "Thumbnail generation failed: {err}"),
            Self::JS(err) => {
                if let Some(err) = err.dyn_ref::<web_sys::js_sys::Error>() {
                    write!(f, "JS error: {}: {}", err.name(), err.message())
                } else if let Some(obj) = err.dyn_ref::<Object>() {
                    write!(f, "JS pseudo-error: {}", obj.to_string())
                } else {
                    write!(f, "JS pseudo-error: {}", make_jsstring(err))
                }
            },
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::error::Error for Error {}

impl ThumbnailWorker {
    pub async fn new() -> Result<ThumbnailWorker, Error> {
        // Create the shared worker
        let worker = SharedWorker::new_with_worker_options(
            "/thumbnails/worker_loader.mjs",
            &{
                let opts = WorkerOptions::new();
                opts.set_name("thumbnails_worker");
                opts.set_type(web_sys::WorkerType::Module);
                opts
            }
        ).map_err(Error::JS)?;
        let port = worker.port();

        // Send it an initial version message
        // This is will force a wait until the worker is ready and will verify
        // that this page and the worker can communicate correctly
        let message = ThumbnailWorkerRequestMessage {
            id: 0,
            request: ThumbnailWorkerRequest::Version { 
                version: built_info::PKG_VERSION.into(),
                git_hash: built_info::GIT_COMMIT_HASH.map(Into::into), 
                git_dirty: built_info::GIT_DIRTY, 
            },
        };
        let message = bincode::serialize(&message).map_err(|e| Error::Bincode(RcEq::new(e)))?;
        let (sender, receiver) = channel::<MessageEvent>();
        let sender_closure = Closure::once(|msg| { let _ = sender.send(msg); });
        port.set_onmessage(Some(sender_closure.as_ref().unchecked_ref()));
        let message = Uint8Array::from(&*message);
        if let Err(err) = port.post_message_with_transferable(&message, &Array::of1(&message.buffer())) {
            port.set_onmessage(None);
            return Err(Error::JS(err));
        }

        let response = select_biased! {
            msg = receiver.fuse() => msg,
            () = sleep(WORKER_INIT_TIMEOUT).fuse() => {
                port.set_onmessage(None);
                return Err(Error::WorkerInitializationTimeout);
            },
        };
        port.set_onmessage(None);
        let response = response.map_err(Error::Cancelled)?.data().dyn_into::<Uint8Array>().map_err(|_| Error::ProtocolError)?;
        let response = bincode::deserialize::<ThumbnailWorkerResponseMessage>(&response.to_vec()).map_err(|e| Error::Bincode(RcEq::new(e)))?;

        // Compare versions
        let ThumbnailWorkerResponse::Version { version, git_hash, git_dirty } = response.response else {
            return Err(Error::ProtocolError);
        };
        if version != built_info::PKG_VERSION || git_hash.as_deref() != built_info::GIT_COMMIT_HASH || git_dirty != built_info::GIT_DIRTY {
            warn!(format!("ThumbnailWorker: Version mismatch detected! Message (de)serialization errors may occur!\nNew client's version: {version}, git hash: {git_hash:?}, git dirty: {git_dirty:?}\nWorker version: {}, git hash: {:?}, git dirty: {:?}\nClose all DeArrow Browser windows to resolve this issue.", built_info::PKG_VERSION, built_info::GIT_COMMIT_HASH, built_info::GIT_DIRTY));
        }

        // it worked, awesome
        // prepare everything for actual use
        let thumb_worker = ThumbnailWorker {
            inner: Rc::new(InnerTW { 
                worker, 
                message_handler: OnceCell::new(), 
                message_queue: RefCell::new(HashMap::new()),
                next_id: Cell::new(0),
                keepalive_interval: OnceCell::new(),
                pagehide_listener: OnceCell::new(),
                protocol_mismatch: Cell::new(false),
            }),
        };

        let handler = thumb_worker.inner.message_handler.get_or_init(|| {
            // use a weak ref, strong ref would cause a reference cycle
            // thumb worker has a ref to the closure, which needs a ref to the worker
            let worker = Rc::downgrade(&thumb_worker.inner);
            Closure::new(move |message| {
                let Some(worker) = worker.upgrade() else {
                    warn!("TW message handler: Failed to upgrade weak thumbworker reference!");
                    return;
                };
                worker.handle_message(message);
            })
        });
        port.set_onmessage(Some(handler.as_ref().unchecked_ref()));

        thumb_worker.inner.keepalive_interval.get_or_init(|| {
            // same as above regarding refs
            let worker = Rc::downgrade(&thumb_worker.inner);
            Interval::new(
                Closure::new(move || {
                    let Some(worker) = worker.upgrade() else {
                        warn!("TW keepalive: Failed to upgrade weak thumbworker reference!");
                        return;
                    };
                    if let Err(err) = worker.post_request(ThumbnailWorkerRequest::Ping) {
                        err.log("TW keepalive: Failed to send ping request");
                    }
                }),
                WORKER_KEEPALIVE_INTERVAL,
            )
        });

        thumb_worker.inner.pagehide_listener.get_or_init(|| {
            let worker = Rc::downgrade(&thumb_worker.inner);
            EventListener::new(
                &window().unwrap(),
                "pagehide",
                Closure::new(move |ev: PageTransitionEvent| {
                    if !ev.persisted() {
                        // we know for sure that we're being unloaded, notify worker if possible
                        let Some(worker) = worker.upgrade() else {
                            warn!("TW pagehide: Failed to upgrade weak thumbworker reference!");
                            return;
                        };
                        if let Err(err) = worker.post_request(ThumbnailWorkerRequest::Disconnecting) {
                            err.log("TW pagehide: Failed to send disconnect notification");
                        }
                    }
                })
            ).expect("should be able to attach an event listener to pagehide")
        });

        Ok(thumb_worker)
    }

    /// Sends a request to the worker without expecting a response
    pub fn post_request(&self, request: ThumbnailWorkerRequest) -> Result<(), Error> {
        self.inner.post_request(request)
    }

    /// Sends a request to the worker and waits for the response
    pub async fn request(&self, request: ThumbnailWorkerRequest) -> Result<ThumbnailWorkerResponse, Error> {
        self.inner.request(request).await
    }

    pub async fn get_thumbnail(&self, key: ThumbnailKey) -> Result<RemoteBlobLink, Error> {
        let ThumbnailWorkerResponse::Thumbnail { r#ref } = self.inner.request(ThumbnailWorkerRequest::GetThumbnail { key }).await? else {
            return Err(Error::ProtocolError);
        };
        let r#ref = r#ref.map_err(Error::Remote)?;
        Ok(RemoteBlobLink {
            worker: self.clone(),
            ref_id: r#ref.ref_id,
            url: r#ref.url,
        })
    }

    pub async fn get_stats(&self) -> Result<ThumbgenStats, Error> {
        let ThumbnailWorkerResponse::Stats { stats } = self.inner.request(ThumbnailWorkerRequest::GetStats).await? else {
            return Err(Error::ProtocolError);
        };
        Ok(stats)
    }

    pub fn is_protocol_mismatched(&self) -> bool {
        self.inner.protocol_mismatch.get()
    }
}

impl InnerTW {
    #[allow(clippy::needless_pass_by_value)] // it's supposed to handle the event by itself
    fn handle_message(&self, message: MessageEvent) {
        let data = match message.data().dyn_into::<Uint8Array>() {
            Ok(data) => data,
            Err(data) => {
                error!("Got a non-Uint8Array message from ThumbnailWorker!", data);
                return;
            },
        };
        let message = match bincode::deserialize::<ThumbnailWorkerResponseMessage>(&data.to_vec()).context("Failed to deserialize a message from ThumbnailWorker!") {
            Ok(msg) => msg,
            Err(error) => {
                self.protocol_mismatch.set(true);
                error!(format!("{error}"));
                return;
            },
        };
        if message.id == 0 {
            if let ThumbnailWorkerResponse::DeserializationError { received_data } = message.response {
                self.protocol_mismatch.set(true);
                warn!("Received a DeserializationError response from ThumbnailWorker!");
                // the message ID on the response is wrong
                // try to deserialize the returned message as a request, and extract the request ID
                // from there
                let request: ThumbnailWorkerRequestMessage = match bincode::deserialize(&received_data).context("Failed to deserialize the returned request message from ThumbnailWorker!") {
                    Ok(m) => m,
                    Err(err) => {
                        // Nothing we can do, report failure and return.
                        error!(format!("{err}"));
                        return;
                    }
                };
                
                // got the initial request back, return error
                let Some(sender) = self.message_queue.borrow_mut().remove(&request.id) else { return };
                let _ = sender.send(Err(Error::ProtocolError));
                return;
            }
        }
        let Some(sender) = self.message_queue.borrow_mut().remove(&message.id) else { 
            self.handle_undelivered_response(message.response);
            return 
        };
        if let Err(Ok(response)) = sender.send(Ok(message.response)) {
            self.handle_undelivered_response(response);
        }
    }

    #[allow(clippy::needless_pass_by_value, clippy::single_match)]
    fn handle_undelivered_response(&self, response: ThumbnailWorkerResponse) {
        match response {
            ThumbnailWorkerResponse::Thumbnail { r#ref: Ok(RawRemoteRef { ref_id, .. }) } => {
                // drop the undelivered ref
                if let Err(err) = self.post_request(ThumbnailWorkerRequest::BlobLinkDropped { ref_id }) {
                    err.log("Failed to notify worker of an undelivered ref");
                }
            },
            _ => (),
        }
    }

    #[allow(clippy::needless_pass_by_value)] // message is not needed anymore
    fn send_message(&self, request: ThumbnailWorkerRequestMessage) -> Result<(), Error> {
        let data = bincode::serialize(&request).map_err(|e| Error::Bincode(RcEq::new(e)))?;
        let data: Uint8Array = (&*data).into();
        self.worker.port().post_message_with_transferable(&data, &Array::of1(&data.buffer())).map_err(Error::JS)?;
        Ok(())
    }

    fn next_id(&self) -> u16 {
        let msg_queue = self.message_queue.borrow();
        assert!(msg_queue.len() < u16::MAX.into(), "Out of message IDs!");
        loop {
            let new_id = self.next_id.replace(self.next_id.get().wrapping_add(1));
            if !msg_queue.contains_key(&new_id) {
                return new_id;
            }
        }
    }
        
    /// Sends a request to the worker without expecting a response
    pub fn post_request(&self, request: ThumbnailWorkerRequest) -> Result<(), Error> {
        let id = self.next_id();
        let request = ThumbnailWorkerRequestMessage { id, request };
        self.send_message(request)
    }

    /// Sends a request to the worker and waits for the response
    pub async fn request(&self, request: ThumbnailWorkerRequest) -> Result<ThumbnailWorkerResponse, Error> {
        let id = self.next_id();
        let (sender, receiver) = channel();
        assert!(self.message_queue.borrow_mut().insert(id, sender).is_none(), "Replaced a message in the queue by mistake!");
        let request = ThumbnailWorkerRequestMessage { id, request };
        self.send_message(request)?;
        receiver.await.map_err(Error::Cancelled).and_then(|r| r)
    }
}

impl Drop for InnerTW {
    fn drop(&mut self) {
        // stop handlers and intervals manually, just in case™
        self.keepalive_interval.stop();
        self.pagehide_listener.stop();
        if let Err(err) = self.post_request(ThumbnailWorkerRequest::Disconnecting) {
            err.log("Failed to notify worker of worker handle being dropped");
        }
    }
}
