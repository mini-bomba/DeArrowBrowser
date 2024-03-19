use std::{future::Future, cell::RefCell};
use std::rc::Rc;
use yew::prelude::*;
use yew::platform::spawn_local;
use yew::suspense::{SuspensionResult, Suspension};


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
