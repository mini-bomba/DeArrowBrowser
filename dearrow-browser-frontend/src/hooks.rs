use std::{future::Future, cell::RefCell};
use std::rc::Rc;
use yew::platform::spawn_local;
use yew::{suspense::{SuspensionResult, Suspension}, hook, use_memo};


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
    let state_ref: Rc<RefCell<UseAsyncSuspensionState<R>>> = use_memo(|_| RefCell::new(UseAsyncSuspensionState::Reset), deps.clone());
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
