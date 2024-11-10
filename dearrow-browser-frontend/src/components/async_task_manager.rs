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

use std::{mem, rc::Rc};

use futures::future::LocalBoxFuture;
use gloo_console::warn;
use slab::Slab;
use yew::{platform::time::sleep, html, AttrValue, Callback, Component, ContextProvider, Html, Properties};

use crate::constants::ASYNC_TASK_AUTO_DISMISS_DELAY;

pub type AsyncTaskFuture = LocalBoxFuture<'static, (AsyncTaskResult, Html)>;

pub enum AsyncTaskResult {
    AutoDismiss {
        success: bool,
    },
    ManualDismiss {
        success: bool,
    },
    DismissOrRetry {
        success: bool,
        retry: Box<dyn FnOnce() -> (AsyncTaskFuture, Html)>,
    },
}

enum InternalAsyncTaskState {
    Pending(),
    Done(AsyncTaskResult),
}

struct InternalAsyncTask {
    name: AttrValue,
    summary: Html,
    state: InternalAsyncTaskState,
}

pub struct AsyncTask {
    pub id: usize,
    pub name: AttrValue,
    pub summary: Html,
    pub state: AsyncTaskState,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum AsyncTaskState {
    Pending {},
    AutoDismiss {
        success: bool,
    },
    ManualDismiss {
        success: bool,
    },
    DismissOrRetry {
        success: bool,
    }
}

impl AsyncTaskState {
    pub fn is_success(&self) -> Option<bool> {
        match self {
            Self::Pending {} => None,
            Self::AutoDismiss { success } | Self::ManualDismiss { success } | Self::DismissOrRetry { success } 
                => Some(*success),
        }
    }
}

pub struct AsyncTaskManager {
    tasks: Slab<InternalAsyncTask>,
    task_order: Vec<usize>,
    control_context: AsyncTaskControl,
    view_context: AsyncTaskList,
}

#[derive(PartialEq, Properties)]
pub struct ATMProps {
    pub children: Html,
}

pub enum ATMControlMessage {
    NewTask {
        name: AttrValue,
        summary: Html,
        task: AsyncTaskFuture,
    },
    DismissTask {
        id: usize,
    },
    RetryTask {
        id: usize,
    },
    DismissAll {},
}

pub enum ATMMessage {
    ControlContextRequest(ATMControlMessage),
    TaskUpdate {
        id: usize,
        result: (AsyncTaskResult, Html),
    },
    AutoDismiss {
        id: usize,
    }
}

#[derive(PartialEq, Clone)]
pub struct AsyncTaskControl {
    callback: Callback<ATMControlMessage>,
}

impl AsyncTaskControl {
    pub fn submit_task(&self, name: AttrValue, summary: Html, task: AsyncTaskFuture) {
        self.callback.emit(ATMControlMessage::NewTask { name, summary, task });
    }
    pub fn dismiss_task(&self, id: usize) {
        self.callback.emit(ATMControlMessage::DismissTask { id });
    }
    pub fn retry_task(&self, id: usize) {
        self.callback.emit(ATMControlMessage::RetryTask { id });
    }
    pub fn dismiss_all(&self) {
        self.callback.emit(ATMControlMessage::DismissAll {});
    }
}

#[derive(Clone)]
pub struct AsyncTaskList {
    pub tasks: Rc<[AsyncTask]>,
}

#[derive(Default, Clone, Copy)]
pub struct TaskCounts {
    pub pending: usize,
    pub success: usize,
    pub failed: usize,
    pub dismissable: usize,
    pub retry: usize,
}

impl PartialEq for AsyncTaskList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.tasks, &other.tasks)
    }
}

impl Eq for AsyncTaskList {}

impl AsyncTaskList {
    pub fn count(&self) -> TaskCounts {
        self.tasks.iter().fold(TaskCounts::default(), |mut counts, task| {
            match task.state {
                AsyncTaskState::Pending {} => counts.pending += 1,
                AsyncTaskState::AutoDismiss { success: true } => counts.success += 1,
                AsyncTaskState::AutoDismiss { success: false } => counts.failed += 1,
                AsyncTaskState::ManualDismiss { success: true } => {
                    counts.success += 1;
                    counts.dismissable += 1;
                },
                AsyncTaskState::ManualDismiss { success: false } => {
                    counts.failed += 1;
                    counts.dismissable += 1;
                },
                AsyncTaskState::DismissOrRetry { success: true } => {
                    counts.success += 1;
                    counts.dismissable += 1;
                    counts.retry += 1;
                },
                AsyncTaskState::DismissOrRetry { success: false } => {
                    counts.failed += 1;
                    counts.dismissable += 1;
                    counts.retry += 1;
                },
            }
            counts
        })
    }
}

impl AsyncTaskManager {
    fn rebuild_view_context(&mut self) {
        let task_list = self.task_order.iter().copied().map(|id| {
            let task = &self.tasks[id];
            AsyncTask {
                id,
                name: task.name.clone(),
                summary: task.summary.clone(),
                state: match task.state {
                    InternalAsyncTaskState::Pending() => AsyncTaskState::Pending {},
                    InternalAsyncTaskState::Done(AsyncTaskResult::AutoDismiss { success }) => AsyncTaskState::AutoDismiss { success },
                    InternalAsyncTaskState::Done(AsyncTaskResult::ManualDismiss { success }) => AsyncTaskState::ManualDismiss { success },
                    InternalAsyncTaskState::Done(AsyncTaskResult::DismissOrRetry { success, .. }) => AsyncTaskState::DismissOrRetry { success },
                }
            }
        }).collect();
        self.view_context = AsyncTaskList { tasks: task_list };
    }
}

impl Component for AsyncTaskManager {
    type Properties = ATMProps;
    type Message = ATMMessage;
    
    fn create(ctx: &yew::Context<Self>) -> Self {
        AsyncTaskManager { 
            tasks: Slab::with_capacity(8),
            task_order: Vec::new(),
            control_context: AsyncTaskControl { callback: ctx.link().callback(ATMMessage::ControlContextRequest) },
            view_context: AsyncTaskList { tasks: Rc::default() },
        }
    }

    fn view(&self, ctx: &yew::Context<Self>) -> Html {
        let props = ctx.props();
        html! {
            <ContextProvider<AsyncTaskControl> context={self.control_context.clone()}>
            <ContextProvider<AsyncTaskList> context={self.view_context.clone()}>
                {props.children.clone()}
            </ContextProvider<AsyncTaskList>>
            </ContextProvider<AsyncTaskControl>>
        }
    }

    fn update(&mut self, ctx: &yew::Context<Self>, msg: Self::Message) -> bool {
        let scope = ctx.link();
        match msg {
            ATMMessage::ControlContextRequest(ATMControlMessage::NewTask { name, summary, task }) => {
                let id = self.tasks.insert(InternalAsyncTask { name, summary, state: InternalAsyncTaskState::Pending() });
                self.task_order.push(id);
                self.rebuild_view_context();
                scope.send_future(async move {
                    ATMMessage::TaskUpdate { 
                        id, 
                        result: task.await, 
                    }
                });
                true
            },
            ATMMessage::ControlContextRequest(ATMControlMessage::DismissTask { id }) => {
                let Some(task) = self.tasks.get(id) else {
                    warn!(format!("Attempted to dismiss a non-existent async task with id {id}"));
                    return false;
                };
                if !matches!(task.state, InternalAsyncTaskState::Done(AsyncTaskResult::ManualDismiss { .. } | AsyncTaskResult::DismissOrRetry { .. })) {
                    warn!(format!("Attempted to dismiss a pending or autodismissed task with id {id}"));
                    return false;
                }
                self.tasks.remove(id);
                self.task_order.retain(|x| *x != id);
                self.rebuild_view_context();
                true
            },
            ATMMessage::ControlContextRequest(ATMControlMessage::RetryTask { id }) => {
                let Some(task) = self.tasks.get_mut(id) else {
                    warn!(format!("Attempted to retry a non-existent async task with id {id}"));
                    return false;
                };
                match task.state {
                    InternalAsyncTaskState::Done(AsyncTaskResult::DismissOrRetry { .. }) => {
                        let InternalAsyncTaskState::Done(AsyncTaskResult::DismissOrRetry { retry, .. }) = mem::replace(&mut task.state, InternalAsyncTaskState::Pending()) else {
                            unreachable!();
                        };
                        let (new_task, summary) = retry();
                        task.summary = summary;
                        task.state = InternalAsyncTaskState::Pending();
                        self.rebuild_view_context();
                        scope.send_future(async move {
                            ATMMessage::TaskUpdate {
                                id,
                                result: new_task.await,
                            }
                        });
                        true
                    },
                    _ => {
                        warn!(format!("Attempted to retry a pending task or a task that cannot be retried (id {id})"));
                        false
                    }
                }
            },
            ATMMessage::ControlContextRequest(ATMControlMessage::DismissAll {}) => {
                let mut removed = Vec::new();
                self.tasks.retain(|i, task| match task.state {
                    InternalAsyncTaskState::Done(AsyncTaskResult::ManualDismiss { .. } | AsyncTaskResult::DismissOrRetry { .. }) => {
                        removed.push(i);
                        false
                    },
                    _ => true,
                });
                if removed.is_empty() {
                    return false;
                }
                self.task_order.retain(|i| !removed.contains(i));
                self.rebuild_view_context();
                true
            }
            ATMMessage::TaskUpdate { id, result: (result, new_summary) } => {
                let Some(task) = self.tasks.get_mut(id) else {
                    warn!(format!("Attempted to update a non-existent async task with id {id}"));
                    return false;
                };
                if !matches!(task.state, InternalAsyncTaskState::Pending()) {
                    warn!(format!("Attempted to update a non-pending async task with id {id}"));
                    return false;
                }
                task.summary = new_summary;
                if matches!(result, AsyncTaskResult::AutoDismiss { .. }) {
                    scope.send_future(async move {
                        sleep(ASYNC_TASK_AUTO_DISMISS_DELAY).await;
                        ATMMessage::AutoDismiss { id }
                    });
                }
                task.state = InternalAsyncTaskState::Done(result);
                self.rebuild_view_context();
                true
            },
            ATMMessage::AutoDismiss { id } => {
                let Some(task) = self.tasks.get(id) else {
                    warn!(format!("Attempted to autodismiss a non-existent async task with id {id}"));
                    return false;
                };
                if !matches!(task.state, InternalAsyncTaskState::Done(AsyncTaskResult::AutoDismiss { .. })) {
                    warn!(format!("Attempted to autodismiss a pending or manually dismissable task with id {id}"));
                    return false;
                }
                self.tasks.remove(id);
                self.task_order.retain(|x| *x != id);
                self.rebuild_view_context();
                true
            }
        }
    }
}
