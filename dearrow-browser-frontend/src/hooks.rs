/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2024 mini_bomba
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
use std::{future::Future, cell::{RefCell, Cell}, rc::Rc};
use yew::prelude::*;
use yew::platform::spawn_local;
use yew::suspense::{SuspensionResult, Suspension};
use yew_router::prelude::*;

use crate::pages::{LocationState, MainRoute};


enum UseAsyncSuspensionState<R>
where
    R: 'static,
{
    Reset,
    Running(Suspension),
    Finished(Rc<R>),
}

#[hook]
pub fn use_async_suspension<FF, F, D, R>(future: FF, deps: D) -> SuspensionResult<Rc<R>> 
where
    FF: 'static + FnOnce(D) -> F,
    F:  'static + Future<Output = R>,
    D:  'static + PartialEq + Clone,
    R:  'static,
{
    let state_ref: Rc<RefCell<UseAsyncSuspensionState<R>>> = use_memo(deps.clone(), |_| RefCell::new(UseAsyncSuspensionState::Reset));
    let mut state = state_ref.borrow_mut();
    match *state {
        UseAsyncSuspensionState::Running(ref sus) => Err(sus.clone()),
        UseAsyncSuspensionState::Finished(ref res) => Ok(res.clone()),
        UseAsyncSuspensionState::Reset => {
            let (sus, sus_handle) = Suspension::new();
            *state = UseAsyncSuspensionState::Running(sus.clone());
            drop(state);
            spawn_local(async move {
                let result = future(deps).await;
                *state_ref.borrow_mut() = UseAsyncSuspensionState::Finished(Rc::new(result));
                sus_handle.resume();
            });
            Err(sus)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LoopControl {
    Continue,
    Terminate,
}

pub struct IterationResult<R, S> {
    pub result: R,
    pub control: LoopControl,
    pub state: S,
}

/// A hook that runs an async function in a loop.
///
/// The function will keep getting called until either it returns LoopControl::Terminate, or the
/// dependencies change.
#[hook]
pub fn use_async_loop<FF, F, D, R, S>(future: FF, deps: D) -> UseStateHandle<R>
where
    FF: 'static + Fn(D, S) -> F,
    F:  'static + Future<Output = IterationResult<R, S>>,
    D:  'static + PartialEq + Clone,
    R:  'static + Default + Clone,
    S:  'static + Default
{

    let result_state: UseStateHandle<R> = use_state(|| Default::default());
    let reset_counter: Rc<Cell<usize>> = use_memo((), |()| Cell::default());
    {
        let result_state = result_state.clone();
        use_memo(deps, move |deps| {
            let deps = deps.clone();
            reset_counter.set(reset_counter.get()+1);
            result_state.set(Default::default());
            let reset_idx = reset_counter.get();
            spawn_local(async move {
                let mut internal_state: S = Default::default();
                loop {
                    let res = future(deps.clone(), internal_state).await;
                    internal_state = res.state;
                    if reset_idx != reset_counter.get() {
                        return;
                    }
                    result_state.set(res.result);
                    if res.control == LoopControl::Terminate {
                        return;
                    }
                }
            });
        });
    }
    result_state
}

#[hook]
pub fn use_memo_state_eq<T, F, D>(deps: D, init_fn: F) -> UseStateHandle<T> 
where
    T: 'static + PartialEq,
    F: Fn() -> T,
    D: 'static + PartialEq + Clone,
{
    let state = use_state_eq(&init_fn);
    {
        // yes, we're using use_memo to reset a state on changes to props
        let state = state.clone();
        use_memo(deps, move |_| {
            state.set(init_fn());
        });
    }
    state
}

#[derive(Clone)]
pub struct LocationStateHandle {
    navigator: Navigator,
    route: MainRoute,
    location: Location,
}

impl LocationStateHandle {
    pub fn get_state(&self) -> LocationState {
        match self.location.state::<LocationState>() {
            Some(state) => *state,
            None => {
                let state = LocationState::default();
                self.replace_state(state);
                state
            }
        }
    }

    pub fn push_state(&self, new_state: LocationState) {
        self.navigator.push_with_state(&self.route, new_state);
    }

    pub fn replace_state(&self, new_state: LocationState) {
        self.navigator.replace_with_state(&self.route, new_state);
    }
}

#[hook]
pub fn use_location_state() -> LocationStateHandle {
    let navigator = use_navigator().expect("Navigator should be present");
    let route = use_route::<MainRoute>().expect("MainRoute should be present");
    let location = use_location().expect("Location should be present");

    LocationStateHandle {
        navigator, route, location
    }
}
