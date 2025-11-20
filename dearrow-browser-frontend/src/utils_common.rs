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

use std::borrow::Cow;
use std::cell::{OnceCell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::ops::Deref;
use std::rc::Rc;
use std::str::Utf8Error;

use cloneable_errors::{bail, ErrContext, ErrorContext, ResContext, SerializableError};
use futures::channel::oneshot;
use gloo_console::error;
use reqwest::Url;
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::{global, Function, JsString, Promise};
use web_sys::{
    window, AbortController, AddEventListenerOptions, EventTarget, Response, Window,
    WorkerGlobalScope,
};

use crate::constants::REQWEST_CLIENT;

// stringifying js values
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = String)]
    pub fn make_jsstring(value: &JsValue) -> JsString;
}


// sleep future
#[wasm_bindgen(inline_js = "export function js_sleep(ms) { return new Promise((res, _) => setTimeout(() => res(), ms)) }")]
extern "C" {
    fn js_sleep(ms: u32) -> Promise;
}

pub async fn sleep(ms: u32) {
    if let Err(err) = JsFuture::from(js_sleep(ms)).await {
        error!("Got a JS error while attempting to sleep", err);
    }
}


// global functions for window & worker use
#[derive(Clone)]
pub enum Global {
    Window(Window),
    Worker(WorkerGlobalScope),
}

impl Global {
    pub fn get() -> Option<Self> {
        if let Some(window) = window() {
            return Some(Self::Window(window));
        }

        let global = global();

        match global.dyn_into::<WorkerGlobalScope>() {
            Ok(worker) => Some(Self::Worker(worker)),
            Err(..) => None,
        }
    }

    pub async fn fetch(&self, url: &str) -> Result<Response, JsValue> {
        let fetch: JsFuture = match self {
            Self::Window(window) => window.fetch_with_str(url),
            Self::Worker(worker) => worker.fetch_with_str(url),
        }.into();
        fetch.await.map(JsCast::unchecked_into)
    }

    pub fn set_timeout(&self, callback: &Function, timeout: i32) -> i32 {
        match self {
            Self::Window(window) => window.set_timeout_with_callback_and_timeout_and_arguments_0(callback, timeout),
            Self::Worker(worker) => worker.set_timeout_with_callback_and_timeout_and_arguments_0(callback, timeout),
        }.unwrap()
    }

    pub fn set_interval(&self, callback: &Function, interval: i32) -> i32 {
        match self {
            Self::Window(window) => window.set_interval_with_callback_and_timeout_and_arguments_0(callback, interval),
            Self::Worker(worker) => worker.set_interval_with_callback_and_timeout_and_arguments_0(callback, interval),
        }.unwrap()
    }

    pub fn clear_interval(&self, handle: i32) {
        match self {
            Self::Window(window) => window.clear_interval_with_handle(handle),
            Self::Worker(worker) => worker.clear_interval_with_handle(handle),
        }
    }
}


/// Represents a registered interval
///
/// Automatically canclled when this object is dropped
pub struct Interval<F: ?Sized> {
    _callback: Closure<F>,
    handle: i32,
}

impl<F: ?Sized> Interval<F> {
    pub fn new(callback: Closure<F>, interal: i32) -> Interval<F> {
        Interval { 
            handle: GLOBAL.with(|g| g.set_interval(callback.as_ref().unchecked_ref(), interal)),
            _callback: callback, 
        }
    }

    pub fn stop(&self) {
        GLOBAL.with(|g| g.clear_interval(self.handle));
    }
}

impl<F: ?Sized> Drop for Interval<F> {
    fn drop(&mut self) {
        self.stop();
    }
}


/// Represents a registered event listener
///
/// Automatically cancelled via the `AbortController` when this object is dropped
pub struct EventListener<F: ?Sized> {
    _closure: Closure<F>,
    abort: AbortController,
}

impl<F: ?Sized> EventListener<F> {
    pub fn new(target: &EventTarget, r#type: &str, handler: Closure<F>) -> Result<Self, JsValue> {
        Self::new_with_options(target, r#type, handler, &AddEventListenerOptions::new())
    }

    pub fn new_with_options(target: &EventTarget, r#type: &str, handler: Closure<F>, options: &AddEventListenerOptions) -> Result<Self, JsValue> {
        let abort = AbortController::new().expect("should be able to construct an AbortController");
        options.set_signal(&abort.signal());
        target.add_event_listener_with_callback_and_add_event_listener_options(r#type, handler.as_ref().unchecked_ref(), options)?;
        Ok(Self {
            _closure: handler,
            abort,
        })
    }

    pub fn stop(&self) {
        self.abort.abort();
    }
}

impl<F: ?Sized> Drop for EventListener<F> {
    fn drop(&mut self) {
        self.stop();
    }
}


/// Extensions for `OnceCell<Interval<F>>` and `OnceCell<EventListener<F>>`
pub trait EventCellsExt {
    fn stop(&self);
}

impl<F: ?Sized> EventCellsExt for OnceCell<Interval<F>> {
    fn stop(&self) {
        if let Some(handler) = self.get() {
            handler.stop();
        }
    }
}

impl<F: ?Sized> EventCellsExt for OnceCell<EventListener<F>> {
    fn stop(&self) {
        if let Some(handler) = self.get() {
            handler.stop();
        }
    }
}

pub struct RateLimiter {
    inner: RefCell<InnerRL>,
}

struct InnerRL {
    current_timeout_id: Option<i32>,
    capacity_used: u8,
    max_capacity: u8,
    queue: VecDeque<oneshot::Sender<()>>,
}

impl InnerRL {
    fn schedule_reset(&mut self) {
        self.current_timeout_id = Some(RESET_LIMITER_FN.with(|rf| GLOBAL.with(|g| g.set_timeout(rf, 100))));
    }
}

impl RateLimiter {
    fn new() -> RateLimiter {
        Self {
            inner: RefCell::new(InnerRL {
                current_timeout_id: None,
                capacity_used: 0,
                max_capacity: 10,
                queue: VecDeque::new(),
            }),
        }
    }

    /// Waits for the next available rate limiter slot
    /// 
    /// The returned future does not borrow self
    pub fn wait(&self) -> impl Future<Output = ()> {
        let mut inner = self.inner.borrow_mut();

        // try to get a slot now
        let queue_signal = if inner.capacity_used < inner.max_capacity {
            // no need to wait
            inner.capacity_used += 1;
            if inner.current_timeout_id.is_none() {
                inner.schedule_reset();
            }
            None
        } else {
            // gotta queue
            let (sender, receiver) = oneshot::channel();
            inner.queue.push_back(sender);
            Some(receiver)
        };

        async move {
            if let Some(signal) = queue_signal {
                signal.await.expect("RateLimiter signal receiver got cancelled");
            }
        }
    }
}

fn reset_rate_limit() {
    RATE_LIMITER.with(|rl| {
        let mut inner = rl.inner.borrow_mut();
        inner.capacity_used = 0;
        inner.current_timeout_id = None;

        while let Some(sender) = inner.queue.pop_front() {
            let _ = sender.send(());
            inner.capacity_used += 1;

            if inner.capacity_used >= inner.max_capacity {
                break;
            }
        }

        if inner.capacity_used != 0 {
            inner.schedule_reset();
        }
    });
}

thread_local! {
    pub static GLOBAL: Global = Global::get().unwrap();
    pub static RATE_LIMITER: RateLimiter = RateLimiter::new();
    static RESET_LIMITER_FN: Function = Closure::<dyn Fn()>::new(reset_rate_limit).into_js_value().unchecked_into();
}

pub trait ReqwestUrlExt {
    #[allow(clippy::result_unit_err)]
    fn extend_segments<I>(&mut self, segments: I) -> Result<&mut Self, ()>
    where I: IntoIterator,
    I::Item: AsRef<str>;
    #[allow(clippy::result_unit_err)]
    fn join_segments<I>(&self, segments: I) -> Result<Self, ()>
    where I: IntoIterator,
    I::Item: AsRef<str>,
    Self: Sized;
}

impl ReqwestUrlExt for Url {
    fn extend_segments<I>(&mut self, segments: I) -> Result<&mut Self, ()>
        where I: IntoIterator,
        I::Item: AsRef<str>,
    {
        {
            let mut path = self.path_segments_mut()?;
            path.extend(segments);
        }
        Ok(self)
    }
    fn join_segments<I>(&self, segments: I) -> Result<Self, ()>
        where I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.clone();
        url.extend_segments(segments)?;
        Ok(url)
    }
}

pub trait ReqwestResponseExt: Sized {
    #[allow(async_fn_in_trait)]  // this is for local use
    async fn check_status(self) -> Result<Self, ErrorContext>;
}

impl ReqwestResponseExt for reqwest::Response {
    async fn check_status(self) -> Result<Self, ErrorContext> {
        let status = self.status();
        if status.is_success() {
            Ok(self)
        } else {
            let body = self.text().await.with_context(|| format!("The server returned a '{status}' status code"))?;
            match serde_json::from_str::<SerializableError>(&body) {
                Err(..) => bail!("The server returned a '{status}' status code with the following body:\n{body}",),
                Ok(err) => Err(err.context("--- SERVER ERROR STACK FOLLOWS ---").context(format!("The server returned a '{status}' status code")))
            }
        }
    }
}

pub async fn api_request<U,R>(url: U) -> Result<R, ErrorContext>
where 
    U: reqwest::IntoUrl,
    R: serde::de::DeserializeOwned,
{
    REQWEST_CLIENT
        .get(url)
        .header("Accept", "application/json")
        .send().await.context("Failed to send the request")?
        .check_status().await?
        .json().await.context("Failed to deserialize response")
}

/// Wrapper type for comparing Rc's via their addresses
pub struct RcEq<T: ?Sized>(pub Rc<T>);

impl<T: ?Sized> PartialEq for RcEq<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl<T: ?Sized> Eq for RcEq<T> {}

impl<T: ?Sized> Deref for RcEq<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: ?Sized> Clone for RcEq<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> From<T> for RcEq<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<I> From<&[I]> for RcEq<[I]> 
where I: Clone,
{
    fn from(value: &[I]) -> Self {
        Self(Rc::from(value))
    }
}

impl<T> RcEq<T> {
    pub fn new(val: T) -> Self {
        Self(Rc::new(val))
    }
}

pub fn url_decode(input: &str) -> Result<Cow<'_, str>, Utf8Error> {
    percent_encoding::percent_decode_str(input).decode_utf8()
}
