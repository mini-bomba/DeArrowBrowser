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

use std::cell::OnceCell;

use gloo_console::error;
use web_sys::{js_sys::{global, Function, JsString, Promise, Reflect}, window, AbortController, AbortSignal, AddEventListenerOptions, EventTarget, Response, Window, WorkerGlobalScope};
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

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
        };
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


/// Extensions for the `web_sys::AddEventListenerOptions` object
pub trait EventListenerOptionsExt {
    fn signal(&mut self, signal: &AbortSignal) -> &mut Self;
}

impl EventListenerOptionsExt for AddEventListenerOptions {
    fn signal(&mut self, signal: &AbortSignal) -> &mut Self {
        Reflect::set(self.as_ref(), &"signal".into(), signal).expect("setting signal property should work");
        self
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
        Self::new_with_options(target, r#type, handler, AddEventListenerOptions::new())
    }

    pub fn new_with_options(target: &EventTarget, r#type: &str, handler: Closure<F>, mut options: AddEventListenerOptions) -> Result<Self, JsValue> {
        let abort = AbortController::new().expect("should be able to construct an AbortController");
        options.signal(&abort.signal());
        target.add_event_listener_with_callback_and_add_event_listener_options(r#type, handler.as_ref().unchecked_ref(), &options)?;
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

thread_local! {
    pub static GLOBAL: Global = Global::get().unwrap();
}
