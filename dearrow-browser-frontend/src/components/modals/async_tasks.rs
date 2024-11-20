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
use yew::prelude::*;
use crate::{components::{async_task_manager::AsyncTaskState, icon::{Icon, IconType}}, contexts::AsyncTaskControl, AsyncTaskList};

fn create_dismiss_callback(async_task_control: AsyncTaskControl, id: usize) -> Callback<MouseEvent> {
    Callback::from(move |_| async_task_control.dismiss_task(id))
}

fn create_retry_callback(async_task_control: AsyncTaskControl, id: usize) -> Callback<MouseEvent> {
    Callback::from(move |_| async_task_control.retry_task(id))
}

#[function_component]
pub fn AsyncTasksModal() -> Html {
    let async_task_control = use_context::<AsyncTaskControl>().unwrap();
    let async_task_view = use_context::<AsyncTaskList>().unwrap();

    let dismiss_all = {
        let async_task_control = async_task_control.clone();
        use_callback((), move |_, ()| {
            async_task_control.dismiss_all();
        })
    };

    let task_counts = async_task_view.count();
    let tasks = async_task_view.tasks.iter().map(|task| {
        html! {
            <div class="async-task-item" key={task.id}>
                <div class="async-task-status">
                    {match task.state.is_success() {
                        None => html! {<Icon r#type={IconType::Wait} tooltip={Some("Task pending")} />},
                        Some(true) => html! {<Icon r#type={IconType::Done} tooltip={Some("Task done")} />},
                        Some(false) => html! {<Icon r#type={IconType::Removed} tooltip={Some("Task failed")} />},
                    }}
                </div>
                <div class="async-task-name">
                    {task.name.clone()}
                </div>
                <div class="async-task-summary">
                    {task.summary.clone()}
                </div>
                <div class="async-task-actions">
                    if let AsyncTaskState::ManualDismiss {..} | AsyncTaskState::DismissOrRetry {..} = task.state {
                        <Icon r#type={IconType::Close} tooltip={Some("Dismiss")} onclick={create_dismiss_callback(async_task_control.clone(), task.id)} />
                    }
                    if let AsyncTaskState::DismissOrRetry {..} = task.state {
                        <Icon r#type={IconType::Replaced} tooltip={Some("Retry")} onclick={create_retry_callback(async_task_control.clone(), task.id)} />
                    }
                </div>
            </div>
        }
    });

    html! {
        <div id="async-tasks-modal">
            <h2>{"Async tasks"}</h2>
            if async_task_view.tasks.is_empty() {
                <div id="async-tasks-no-tasks">{"No async tasks"}</div>
            } else {
                if task_counts.dismissable > 0 {
                    <div id="async-tasks-controls">
                        <button onclick={dismiss_all}>{"Dismiss all"}</button>
                    </div>
                }
                {for tasks}
            }
        </div>
    }
}
