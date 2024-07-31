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

use std::{fmt::Display, num::NonZeroUsize, rc::Rc, str::FromStr};

use reqwest::Url;
use strum::VariantNames;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use gloo_console::log;

use crate::{contexts::SettingsContext, settings::TableLayout};

/// Generator macro for a revert callback (Esc key pressed)
///
/// Takes in the name of the settings field and a function to verify the input field's value
macro_rules! esc_callback {
    ($name:ident, $verify_func:expr) => {
        move |e: KeyboardEvent, settings_context| {
            if e.key() == "Escape" {
                let settings = settings_context.settings();
                let target: HtmlInputElement = e.target_unchecked_into();
                target.set_value(&settings.$name.to_string());
                if $verify_func(&target).is_none() {
                    target.set_value(&settings_context.default.$name.to_string());
                    assert!($verify_func(&target).is_some(), stringify!(Default value of $name setting was invalid!));
                }
            }
        }
    };
}

/// Generator macro for the undo button callback
///
/// Takes in the name of the settings field
macro_rules! undo_callback {
    ($name:ident) => {
        move |_: MouseEvent, (settings_context, initial_settings)| {
            let mut settings = settings_context.settings().clone();
            settings.$name = initial_settings.$name.clone();
            settings_context.update(settings);
        }
    };
}

/// Generator macro for a check to determine whether the undo button should be visible
///
/// Takes in the name of the settings field, the current and the initial settings
macro_rules! should_show_undo {
    ($name: ident, $current_settings:expr, $initial_settings:expr) => {
        $current_settings.$name != $initial_settings.$name
    };
}

/// Generator macro for the reset to default button callback
///
/// Takes in the name of the settings field
macro_rules! reset_callback {
    ($name:ident) => {
        move |_: MouseEvent, settings_context| {
            let mut settings = settings_context.settings().clone();
            settings.$name = settings_context.default.$name.clone();
            settings_context.update(settings);
        }
    };
}

/// Generator macro for a check to determine whether the reset button should be visible
///
/// Takes in the name of the settings field, the current and the settings context (for default
/// settings)
macro_rules! should_show_reset {
    ($name: ident, $current_settings:expr, $settings_context:expr) => {
        $current_settings.$name != $settings_context.default.$name
    };
}

/// Generator macro for a save callback (change committed)
///
/// Takes in the name of the settings field, a function to verify & parse the input field's value
macro_rules! save_callback {
    ($name:ident, $verify_func:ident) => {
        move |e: Event, settings_context| {
            let target: HtmlInputElement = e.target_unchecked_into();
            if let Some(v) = $verify_func(&target) {
                log!(format!("Saved! {v:?}"));
                let mut settings = settings_context.settings().clone();
                settings.$name = v;
                settings_context.update(settings);
            }
        }
    };
}

/// Generator macro for input field validation & parsing functions
///
/// Takes the name of the new function, the return type and a code block that does additional verification,
/// should the JS verification pass.
macro_rules! verify_fn {
    ($name:ident: $target:ident -> $type:ty => $check:block) => {
        fn $name($target: &HtmlInputElement) -> Option<$type> {
            let mut res = None;
            $target.set_custom_validity("");
            if $target.validity().valid() {
                res = match $check {
                    Err(e) => {
                        $target.set_custom_validity(&format!("{e}"));
                        None
                    },
                    Ok(v) => Some(v),
                }
            }
            $target.report_validity().then_some(res).flatten()
        }
    };
}

macro_rules! setting_class {
    ($initial_settings:expr, $current_settings:expr, $name:ident) => {
        if $initial_settings.$name != $current_settings.$name {
            classes!("setting-changed")
        } else {
            classes!()
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BaseUrlVerifyError {
    UrlParseError(<Url as FromStr>::Err),
    CannotBeABase,
    InvalidScheme,
}

impl std::error::Error for BaseUrlVerifyError {}
impl Display for BaseUrlVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UrlParseError(ref e) => write!(f, "{e}"),
            Self::CannotBeABase => write!(f, "This URL cannot be a base"),
            Self::InvalidScheme => write!(f, "Invalid scheme - only the https: scheme is accepted"),
        }
    }
}


fn fromstr_verify<T>(target: &HtmlInputElement) -> Option<T> 
where T: FromStr,
      T::Err: Display,
{
    let mut res = None;
    target.set_custom_validity("");
    if target.validity().valid() {
        res = match FromStr::from_str(&target.value()) {
            Err(e) => {
                target.set_custom_validity(&format!("{e}"));
                None
            },
            Ok(v) => Some(v),
        }
    }
    target.report_validity().then_some(res).flatten()
}
verify_fn!(baseurl_verify: target -> Rc<str> => {
    match Url::from_str(&target.value()) {
        Err(e) => Err(BaseUrlVerifyError::UrlParseError(e)),
        Ok(url) => {
            if url.cannot_be_a_base() {
                Err(BaseUrlVerifyError::CannotBeABase)
            } else if url.scheme() != "https" {
                Err(BaseUrlVerifyError::InvalidScheme)
            } else {
                Ok(url.to_string().into())
            }
        },
    }
});

fn update_select<T>(input: &(NodeRef, T))
where T: Into<&'static str> + Clone
{
    if let Some(r#ref) = input.0.cast::<HtmlSelectElement>() {
        r#ref.set_value(input.1.clone().into());
    }
}

#[function_component]
pub fn SettingsModal() -> Html {
    let settings_context: SettingsContext = use_context().expect("SettingsContext should be available");
    let initial_settings = use_memo((), |()| settings_context.settings().clone());
    let current_settings = settings_context.settings();

    let entries_per_page_ref = use_node_ref();
    let thumbgen_api_base_url_ref = use_node_ref();
    let title_table_layout_ref = use_node_ref();
    let thumbnail_table_layout_ref = use_node_ref();

    let nonzerousize_oninput = use_callback((), move |e: InputEvent, ()| {
        fromstr_verify::<NonZeroUsize>(&e.target_unchecked_into());
    });
    let baseurl_oninput = use_callback((), move |e: InputEvent, ()| {
        baseurl_verify(&e.target_unchecked_into());
    });
    // let tablelayout_oninput = use_callback((), move |e: InputEvent, ()| {
    //     fromstr_verify::<TableLayout>(&e.target_unchecked_into());
    // });

    let entries_per_page_revert      = use_callback(settings_context.clone(), esc_callback!(entries_per_page, fromstr_verify::<NonZeroUsize>));
    let thumbgen_api_base_url_revert = use_callback(settings_context.clone(), esc_callback!(thumbgen_api_base_url, baseurl_verify));

    let entries_per_page_save       = use_callback(settings_context.clone(), save_callback!(entries_per_page, fromstr_verify));
    let thumbgen_api_base_url_save  = use_callback(settings_context.clone(), save_callback!(thumbgen_api_base_url, baseurl_verify));
    let title_table_layout_save     = use_callback(settings_context.clone(), save_callback!(title_table_layout, fromstr_verify));
    let thumbnail_table_layout_save = use_callback(settings_context.clone(), save_callback!(thumbnail_table_layout, fromstr_verify));

    let entries_per_page_undo       = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(entries_per_page));
    let thumbgen_api_base_url_undo  = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(thumbgen_api_base_url));
    let title_table_layout_undo     = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(title_table_layout));
    let thumbnail_table_layout_undo = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(thumbnail_table_layout));

    let entries_per_page_reset       = use_callback(settings_context.clone(), reset_callback!(entries_per_page));
    let thumbgen_api_base_url_reset  = use_callback(settings_context.clone(), reset_callback!(thumbgen_api_base_url));
    let title_table_layout_reset     = use_callback(settings_context.clone(), reset_callback!(title_table_layout));
    let thumbnail_table_layout_reset = use_callback(settings_context.clone(), reset_callback!(thumbnail_table_layout));

    // ~value doesnt work for <select>
    use_effect_with((title_table_layout_ref.clone(), current_settings.title_table_layout), update_select);
    use_effect_with((thumbnail_table_layout_ref.clone(), current_settings.thumbnail_table_layout), update_select);

    html! {
        <div id="settings-modal">
            <h2>{"DeArrow Browser Settings"}</h2>
            <fieldset>
                <legend>{"Site appearance"}</legend>
                <label for="entries_per_page">{"Detail entries per page: "}</label>
                <input 
                    class={setting_class!(initial_settings, current_settings, entries_per_page)} 
                    id="entries_per_page" 
                    type="number" step=1 min=1 required=true 
                    oninput={nonzerousize_oninput} 
                    onkeydown={entries_per_page_revert} 
                    onchange={entries_per_page_save} 
                    ref={entries_per_page_ref}
                    ~value={current_settings.entries_per_page.to_string()} 
                />
                <div class="setting-actions">
                    if should_show_undo!(entries_per_page, current_settings, initial_settings) {
                        <span 
                            class="clickable" title="Undo"
                            onclick={entries_per_page_undo}
                        >{"↩️"}</span>
                    }
                    if should_show_reset!(entries_per_page, current_settings, settings_context) {
                        <span 
                            class="clickable" title="Reset to default"
                            onclick={entries_per_page_reset}
                        >{"🔄"}</span>
                    }
                </div>
                <label for="title_table_layout">{"Title table layout: "}</label>
                <select 
                    id="title_table_layout"
                    class={setting_class!(initial_settings, current_settings, title_table_layout)} 
                    onchange={title_table_layout_save}
                    ref={title_table_layout_ref}
                >
                    {for TableLayout::VARIANTS.iter().map(|&name| html!{ <option key={name}>{name}</option> })}
                </select>
                <div class="setting-actions">
                    if should_show_undo!(title_table_layout, current_settings, initial_settings) {
                        <span 
                            class="clickable" title="Undo"
                            onclick={title_table_layout_undo}
                        >{"↩️"}</span>
                    }
                    if should_show_reset!(title_table_layout, current_settings, settings_context) {
                        <span 
                            class="clickable" title="Reset to default"
                            onclick={title_table_layout_reset}
                        >{"🔄"}</span>
                    }
                </div>
                <label for="thumbnail_table_layout">{"Thumbnail table layout: "}</label>
                <select 
                    id="thumbnail_table_layout"
                    class={setting_class!(initial_settings, current_settings, thumbnail_table_layout)} 
                    onchange={thumbnail_table_layout_save}
                    ref={thumbnail_table_layout_ref}
                >
                    {for TableLayout::VARIANTS.iter().map(|&name| html!{ <option key={name}>{name}</option> })}
                </select>
                <div class="setting-actions">
                    if should_show_undo!(thumbnail_table_layout, current_settings, initial_settings) {
                        <span 
                            class="clickable" title="Undo"
                            onclick={thumbnail_table_layout_undo}
                        >{"↩️"}</span>
                    }
                    if should_show_reset!(thumbnail_table_layout, current_settings, settings_context) {
                        <span 
                            class="clickable" title="Reset to default"
                            onclick={thumbnail_table_layout_reset}
                        >{"🔄"}</span>
                    }
                </div>
            </fieldset>
            <fieldset>
                <legend>{"Thumbnail generator"}</legend>
                <label for="thumbgen_api_base_url">{"Thumbnail cache API base URL: "}</label>
                <input 
                    class={setting_class!(initial_settings, current_settings, thumbgen_api_base_url)} 
                    id="thumbgen_api_base_url" 
                    type="url" required=true 
                    oninput={baseurl_oninput} 
                    onkeydown={thumbgen_api_base_url_revert} 
                    onchange={thumbgen_api_base_url_save} 
                    ref={thumbgen_api_base_url_ref}
                    ~value={current_settings.thumbgen_api_base_url.to_string()} 
                />
                <div class="setting-actions">
                    if should_show_undo!(thumbgen_api_base_url, current_settings, initial_settings) {
                        <span 
                            class="clickable" title="Undo"
                            onclick={thumbgen_api_base_url_undo}
                        >{"↩️"}</span>
                    }
                    if should_show_reset!(thumbgen_api_base_url, current_settings, settings_context) {
                        <span 
                            class="clickable" title="Reset to default"
                            onclick={thumbgen_api_base_url_reset}
                        >{"🔄"}</span>
                    }
                </div>
            </fieldset>
        </div>
    }
}
