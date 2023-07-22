use std::{future::Future, cell::RefCell};
use std::rc::Rc;
use yew::platform::spawn_local;
use yew::{suspense::{SuspensionResult, Suspension}, hook, use_memo};


enum UseAsyncWithDepsState<R>
where
    R: 'static,
{
    Reset,
    Running(Suspension),
    Finished(Rc<R>),
}

#[hook]
pub fn use_async_with_deps<FF, F, D, R>(future: FF, deps: D) -> SuspensionResult<Rc<R>> 
where
    FF: 'static + FnOnce(D) -> F,
    F:  'static + Future<Output = R>,
    D:  'static + PartialEq + Clone,
    R:  'static,
{
    let state_ref: Rc<RefCell<UseAsyncWithDepsState<R>>> = use_memo(|_| RefCell::new(UseAsyncWithDepsState::Reset), deps.clone());
    let mut state = state_ref.borrow_mut();
    match *state {
        UseAsyncWithDepsState::Running(ref sus) => Err(sus.clone()),
        UseAsyncWithDepsState::Finished(ref res) => Ok(res.clone()),
        UseAsyncWithDepsState::Reset => {
            let (sus, sus_handle) = Suspension::new();
            *state = UseAsyncWithDepsState::Running(sus.clone());
            drop(state);
            spawn_local(async move {
                let result = future(deps).await;
                *state_ref.borrow_mut() = UseAsyncWithDepsState::Finished(Rc::new(result));
                sus_handle.resume();
            });
            Err(sus)
        }
    }
}
