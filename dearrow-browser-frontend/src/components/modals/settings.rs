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

use std::{convert::Infallible, fmt::Display, num::NonZeroUsize, rc::Rc, str::FromStr};

use gloo_console::error;
use reqwest::Url;
use strum::VariantNames;
use wasm_bindgen::JsCast;
use web_sys::{ClipboardEvent, HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

use crate::{contexts::settings::SettingsContext, settings::{TableLayout, ThemeVariant}};

const DISABLE_SW_TITLE: &str = "This is meant for debugging only - this disables sharing the thumbnail cache between all open tabs and makes the current tab handle all thumbnail fetching on it's own. Changes require a refresh to apply";
const AUTOSEARCH_TITLE: &str = "If enabled, pasting valid query data or URLs into search fields will immediately trigger the search";
const STICKY_HEADERS_TITLE: &str = "This makes all headers sticky (stick to the top of the page as you scroll) including the page header and the table header";

/// Generator macro for a revert callback (Esc key pressed)
///
/// Takes in the name of the settings field and a function to verify the input field's value
macro_rules! esc_callback {
    ($name:ident $(.$subprop:ident)*, $verify_func:expr) => {
        move |e: KeyboardEvent, settings_context| {
            if e.key() == "Escape" {
                let settings = settings_context.settings();
                let target: HtmlInputElement = e.target_unchecked_into();
                target.set_value(&settings.$name$(.$subprop)*.to_string());
                if $verify_func(&target).is_none() {
                    target.set_value(&settings_context.default.$name$(.$subprop)*.to_string());
                    assert!($verify_func(&target).is_some(), stringify!(Default value of $name$(.$subprop)* setting was invalid!));
                }
            }
        }
    };
}

/// Generator macro for the undo button callback
///
/// Takes in the name of the settings field
macro_rules! undo_callback {
    ($name:ident $(.$subprop:ident)*) => {
        move |_: MouseEvent, (settings_context, initial_settings)| {
            let mut settings = settings_context.settings().clone();
            settings.$name$(.$subprop)* = initial_settings.$name$(.$subprop)*.clone();
            settings_context.update(settings);
        }
    };
}

/// Generator macro for a check to determine whether the undo button should be visible
///
/// Takes in the name of the settings field, the current and the initial settings
macro_rules! should_show_undo {
    ($name:ident $(.$subprop:ident)*, $current_settings:expr, $initial_settings:expr) => {
        $current_settings.$name$(.$subprop)* != $initial_settings.$name$(.$subprop)*
    };
    ($name:ident $(.$subprop:ident)*, $current_settings:expr, $initial_settings:expr, $epsilon:literal) => {
        ($initial_settings.$name$(.$subprop)* - $current_settings.$name$(.$subprop)*).abs() >= $epsilon
    };
}

/// Generator macro for the reset to default button callback
///
/// Takes in the name of the settings field
macro_rules! reset_callback {
    ($name:ident $(.$subprop:ident)*) => {
        move |_: MouseEvent, settings_context| {
            let mut settings = settings_context.settings().clone();
            settings.$name$(.$subprop)* = settings_context.default.$name$(.$subprop)*.clone();
            settings_context.update(settings);
        }
    };
}

/// Generator macro for a check to determine whether the reset button should be visible
///
/// Takes in the name of the settings field, the current and the settings context (for default
/// settings)
macro_rules! should_show_reset {
    ($name:ident $(.$subprop:ident)*, $current_settings:expr, $settings_context:expr) => {
        $current_settings.$name$(.$subprop)* != $settings_context.default.$name$(.$subprop)*
    };
    ($name:ident $(.$subprop:ident)*, $current_settings:expr, $settings_context:expr, $epsilon:literal) => {
        ($current_settings.$name$(.$subprop)* - $settings_context.default.$name$(.$subprop)*).abs() >= $epsilon
    };
}

/// Generator macro for a save callback (change committed)
///
/// Takes in the name of the settings field, a function to verify & parse the input field's value
macro_rules! save_callback {
    ($name:ident $(.$subprop:ident)*, $verify_func:ident) => {
        move |e: Event, settings_context| {
            let target: HtmlInputElement = e.target_unchecked_into();
            if let Some(v) = $verify_func(&target) {
                let mut settings = settings_context.settings().clone();
                settings.$name$(.$subprop)* = v;
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
    ($initial_settings:expr, $current_settings:expr, $name:ident $(.$subprop:ident)*) => {
        if should_show_undo!($name$(.$subprop)*, $current_settings, $initial_settings) {
            classes!("setting-changed")
        } else {
            classes!()
        }
    };
    ($initial_settings:expr, $current_settings:expr, $name:ident $(.$subprop:ident)*, $epsilon:literal) => {
        if should_show_undo!($name$(.$subprop)*, $current_settings, $initial_settings, $epsilon) {
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
verify_fn!(checkbox_verify: target -> bool => {
    Result::<bool, Infallible>::Ok(target.checked())
});
verify_fn!(priv_userid_verify: target -> Option<Rc<str>> => {
    let userid = target.value();
    if userid.is_empty() {
        Ok(None)
    } else if userid.len() < 30 {
        Err("Private userID too short")
    } else {
        Ok(Some(userid.into()))
    }
});
verify_fn!(hue_verify: target -> f64 => {
    target.value().parse::<f64>().map(|v| v % 360.)
});
verify_fn!(saturation_verify: target -> f64 => {
    target.value().parse::<f64>().map(|v| v.clamp(0., 100.))
});

fn update_select<T>(input: &(NodeRef, T))
where T: Into<&'static str> + Clone
{
    if let Some(r#ref) = input.0.cast::<HtmlSelectElement>() {
        r#ref.set_value(input.1.clone().into());
    }
}

trait ToStringExt {
    fn to_string(&self) -> String;
}

impl<T: ToString> ToStringExt for Option<T> {
    fn to_string(&self) -> String {
        match self {
            None => String::new(),
            Some(t) => t.to_string(),
        }
    }
}

#[component]
pub fn SettingsModal() -> Html {
    let settings_context: SettingsContext = use_context().expect("SettingsContext should be available");
    let initial_settings = use_memo((), |()| settings_context.settings().clone());
    let current_settings = settings_context.settings();

    let title_table_layout_ref = use_node_ref();
    let thumbnail_table_layout_ref = use_node_ref();
    let theme_variant_ref = use_node_ref();

    let nonzerousize_oninput = use_callback((), move |e: InputEvent, ()| {
        fromstr_verify::<NonZeroUsize>(&e.target_unchecked_into());
    });
    let baseurl_oninput = use_callback((), move |e: InputEvent, ()| {
        baseurl_verify(&e.target_unchecked_into());
    });
    let private_user_id_oninput = use_callback((), move |e: InputEvent, ()| {
        priv_userid_verify(&e.target_unchecked_into());
    });
    let hue_oninput = use_callback((), move |e: InputEvent, ()| {
        hue_verify(&e.target_unchecked_into());
    });
    let saturation_oninput = use_callback((), move |e: InputEvent, ()| {
        saturation_verify(&e.target_unchecked_into());
    });

    let password_copy = use_callback((), move |e: Event, ()| {
        let e: ClipboardEvent = e.dyn_into().expect("This should be a ClipboardEvent");
        let input: HtmlInputElement = e.target_unchecked_into();
        if let Err(err) = e.clipboard_data().expect("Clipboard data should be defined on ClipboardEvents fired by the browser")
                           .set_data("text/plain", &input.value()) {
            error!(".set_data() on a clipboard event failed lolwut", err);
        } else {
            e.prevent_default();
        }
    });

    let entries_per_page_revert           = use_callback(settings_context.clone(), esc_callback!(entries_per_page, fromstr_verify::<NonZeroUsize>));
    let thumbgen_api_base_url_revert      = use_callback(settings_context.clone(), esc_callback!(thumbgen_api_base_url, baseurl_verify));
    let private_user_id_revert            = use_callback(settings_context.clone(), esc_callback!(private_user_id, priv_userid_verify));
    let sponsorblock_api_base_url_revert  = use_callback(settings_context.clone(), esc_callback!(sponsorblock_api_base_url, baseurl_verify));
    let theme_hue_revert                  = use_callback(settings_context.clone(), esc_callback!(theme.hue, hue_verify));
    let theme_saturation_revert           = use_callback(settings_context.clone(), esc_callback!(theme.saturation, saturation_verify));

    let entries_per_page_save             = use_callback(settings_context.clone(), save_callback!(entries_per_page, fromstr_verify));
    let thumbgen_api_base_url_save        = use_callback(settings_context.clone(), save_callback!(thumbgen_api_base_url, baseurl_verify));
    let title_table_layout_save           = use_callback(settings_context.clone(), save_callback!(title_table_layout, fromstr_verify));
    let thumbnail_table_layout_save       = use_callback(settings_context.clone(), save_callback!(thumbnail_table_layout, fromstr_verify));
    let render_thumbnails_in_tables_save  = use_callback(settings_context.clone(), save_callback!(render_thumbnails_in_tables, checkbox_verify));
    let show_original_titles_save         = use_callback(settings_context.clone(), save_callback!(show_original_titles, checkbox_verify));
    let sticky_headers_save               = use_callback(settings_context.clone(), save_callback!(sticky_headers, checkbox_verify));
    let enable_autosearch_save            = use_callback(settings_context.clone(), save_callback!(enable_autosearch, checkbox_verify));
    let disable_sharedworker_save         = use_callback(settings_context.clone(), save_callback!(disable_sharedworker, checkbox_verify));
    let private_user_id_save              = use_callback(settings_context.clone(), save_callback!(private_user_id, priv_userid_verify));
    let sponsorblock_api_base_url_save    = use_callback(settings_context.clone(), save_callback!(sponsorblock_api_base_url, baseurl_verify));
    let theme_enable_save                 = use_callback(settings_context.clone(), save_callback!(theme.enable, checkbox_verify));
    let theme_hue_save                    = use_callback(settings_context.clone(), save_callback!(theme.hue, hue_verify));
    let theme_saturation_save             = use_callback(settings_context.clone(), save_callback!(theme.saturation, saturation_verify));
    let theme_variant_save                = use_callback(settings_context.clone(), save_callback!(theme.variant, fromstr_verify));

    let entries_per_page_undo             = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(entries_per_page));
    let thumbgen_api_base_url_undo        = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(thumbgen_api_base_url));
    let title_table_layout_undo           = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(title_table_layout));
    let thumbnail_table_layout_undo       = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(thumbnail_table_layout));
    let render_thumbnails_in_tables_undo  = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(render_thumbnails_in_tables));
    let show_original_titles_undo         = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(show_original_titles));
    let sticky_headers_undo               = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(sticky_headers));
    let enable_autosearch_undo            = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(enable_autosearch));
    let disable_sharedworker_undo         = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(disable_sharedworker));
    let private_user_id_undo              = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(private_user_id));
    let sponsorblock_api_base_url_undo    = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(sponsorblock_api_base_url));
    let theme_enable_undo                 = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(theme.enable));
    let theme_hue_undo                    = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(theme.hue));
    let theme_saturation_undo             = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(theme.saturation));
    let theme_variant_undo                = use_callback((settings_context.clone(), initial_settings.clone()), undo_callback!(theme.variant));

    let entries_per_page_reset            = use_callback(settings_context.clone(), reset_callback!(entries_per_page));
    let thumbgen_api_base_url_reset       = use_callback(settings_context.clone(), reset_callback!(thumbgen_api_base_url));
    let title_table_layout_reset          = use_callback(settings_context.clone(), reset_callback!(title_table_layout));
    let thumbnail_table_layout_reset      = use_callback(settings_context.clone(), reset_callback!(thumbnail_table_layout));
    let render_thumbnails_in_tables_reset = use_callback(settings_context.clone(), reset_callback!(render_thumbnails_in_tables));
    let show_original_titles_reset        = use_callback(settings_context.clone(), reset_callback!(show_original_titles));
    let sticky_headers_reset              = use_callback(settings_context.clone(), reset_callback!(sticky_headers));
    let enable_autosearch_reset           = use_callback(settings_context.clone(), reset_callback!(enable_autosearch));
    let disable_sharedworker_reset        = use_callback(settings_context.clone(), reset_callback!(disable_sharedworker));
    let private_user_id_reset             = use_callback(settings_context.clone(), reset_callback!(private_user_id));
    let sponsorblock_api_base_url_reset   = use_callback(settings_context.clone(), reset_callback!(sponsorblock_api_base_url));
    let theme_enable_reset                = use_callback(settings_context.clone(), reset_callback!(theme.enable));
    let theme_hue_reset                   = use_callback(settings_context.clone(), reset_callback!(theme.hue));
    let theme_saturation_reset            = use_callback(settings_context.clone(), reset_callback!(theme.saturation));
    let theme_variant_reset               = use_callback(settings_context.clone(), reset_callback!(theme.variant));


    // ~value doesnt work for <select>
    use_effect_with((title_table_layout_ref.clone(),     current_settings.title_table_layout), update_select);
    use_effect_with((thumbnail_table_layout_ref.clone(), current_settings.thumbnail_table_layout), update_select);
    use_effect_with((theme_variant_ref.clone(),          current_settings.theme.variant), update_select);

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
                    for name in TableLayout::VARIANTS {
                        <option key={*name}>{*name}</option>
                    }
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
                if current_settings.title_table_layout == TableLayout::Expanded {
                    <label for="show_original_titles">{"Show original titles in tables: "}</label>
                    <input 
                        class={setting_class!(initial_settings, current_settings, show_original_titles)}
                        id="show_original_titles" 
                        type="checkbox"
                        onchange={show_original_titles_save} 
                        ~checked={current_settings.show_original_titles} 
                    />
                    <div class="setting-actions">
                        if should_show_undo!(show_original_titles, current_settings, initial_settings) {
                            <span 
                                class="clickable" title="Undo"
                                onclick={show_original_titles_undo}
                            >{"↩️"}</span>
                        }
                        if should_show_reset!(show_original_titles, current_settings, settings_context) {
                            <span 
                                class="clickable" title="Reset to default"
                                onclick={show_original_titles_reset}
                            >{"🔄"}</span>
                        }
                    </div>
                }
                <label for="thumbnail_table_layout">{"Thumbnail table layout: "}</label>
                <select 
                    id="thumbnail_table_layout"
                    class={setting_class!(initial_settings, current_settings, thumbnail_table_layout)} 
                    onchange={thumbnail_table_layout_save}
                    ref={thumbnail_table_layout_ref}
                >
                    for name in TableLayout::VARIANTS {
                        <option key={*name}>{*name}</option>
                    }
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
                if current_settings.thumbnail_table_layout == TableLayout::Expanded {
                    <label for="render_thumbnails_in_tables">{"Render thumbnails in tables: "}</label>
                    <input 
                        class={setting_class!(initial_settings, current_settings, render_thumbnails_in_tables)} 
                        id="render_thumbnails_in_tables" 
                        type="checkbox"
                        onchange={render_thumbnails_in_tables_save} 
                        ~checked={current_settings.render_thumbnails_in_tables} 
                    />
                    <div class="setting-actions">
                        if should_show_undo!(render_thumbnails_in_tables, current_settings, initial_settings) {
                            <span 
                                class="clickable" title="Undo"
                                onclick={render_thumbnails_in_tables_undo}
                            >{"↩️"}</span>
                        }
                        if should_show_reset!(render_thumbnails_in_tables, current_settings, settings_context) {
                            <span 
                                class="clickable" title="Reset to default"
                                onclick={render_thumbnails_in_tables_reset}
                            >{"🔄"}</span>
                        }
                    </div>
                }
                <label for="sticky_headers" title={STICKY_HEADERS_TITLE}>{"Sticky headers: "}</label>
                <input 
                    class={setting_class!(initial_settings, current_settings, sticky_headers)} 
                    id="sticky_headers" 
                    type="checkbox"
                    onchange={sticky_headers_save} 
                    ~checked={current_settings.sticky_headers} 
                />
                <div class="setting-actions">
                    if should_show_undo!(sticky_headers, current_settings, initial_settings) {
                        <span 
                            class="clickable" title="Undo"
                            onclick={sticky_headers_undo}
                        >{"↩️"}</span>
                    }
                    if should_show_reset!(sticky_headers, current_settings, settings_context) {
                        <span 
                            class="clickable" title="Reset to default"
                            onclick={sticky_headers_reset}
                        >{"🔄"}</span>
                    }
                </div>
            </fieldset>
            <fieldset>
                <legend>{"Theming"}</legend>
                <label for="theme_enable">{"Enable custom HSL theming: "}</label>
                <input 
                    class={setting_class!(initial_settings, current_settings, theme.enable)} 
                    id="theme_enable" 
                    type="checkbox"
                    onchange={theme_enable_save} 
                    ~checked={current_settings.theme.enable} 
                />
                <div class="setting-actions">
                    if should_show_undo!(theme.enable, current_settings, initial_settings) {
                        <span 
                            class="clickable" title="Undo"
                            onclick={theme_enable_undo}
                        >{"↩️"}</span>
                    }
                    if should_show_reset!(theme.enable, current_settings, settings_context) {
                        <span 
                            class="clickable" title="Reset to default"
                            onclick={theme_enable_reset}
                        >{"🔄"}</span>
                    }
                </div>
                if current_settings.theme.enable {
                    <label for="theme_hue">{"Theme hue: "}</label>
                    <div class="setting-slider">
                        <input 
                            class={classes!(setting_class!(initial_settings, current_settings, theme.hue, 0.001), "custom-slider")}
                            id="theme_hue_slider"
                            type="range"
                            min="0"
                            max="360"
                            step="0.001"
                            onchange={&theme_hue_save} 
                            ~value={current_settings.theme.hue.to_string()} 
                        />
                        <input 
                            class={setting_class!(initial_settings, current_settings, theme.hue, 0.001)} 
                            id="theme_hue" 
                            type="number"
                            min="0"
                            max="360"
                            step="0.001"
                            oninput={hue_oninput} 
                            onkeydown={theme_hue_revert} 
                            onchange={theme_hue_save} 
                            ~value={current_settings.theme.hue.to_string()} 
                        />
                    </div>
                    <div class="setting-actions">
                        if should_show_undo!(theme.hue, current_settings, initial_settings, 0.001) {
                            <span 
                                class="clickable" title="Undo"
                                onclick={theme_hue_undo}
                            >{"↩️"}</span>
                        }
                        if should_show_reset!(theme.hue, current_settings, settings_context, 0.001) {
                            <span 
                                class="clickable" title="Reset to default"
                                onclick={theme_hue_reset}
                            >{"🔄"}</span>
                        }
                    </div>
                    <label for="theme_saturation">{"Theme saturation: "}</label>
                    <div class="setting-slider">
                        <input 
                            class={classes!(setting_class!(initial_settings, current_settings, theme.saturation, 0.001), "custom-slider")}
                            id="theme_saturation_slider" 
                            type="range"
                            min="0"
                            max="100"
                            step="0.001"
                            onchange={&theme_saturation_save} 
                            ~value={current_settings.theme.saturation.to_string()} 
                        />
                        <input 
                            class={setting_class!(initial_settings, current_settings, theme.saturation, 0.001)} 
                            id="theme_saturation" 
                            type="number"
                            min="0"
                            max="100"
                            step="0.001"
                            oninput={saturation_oninput} 
                            onkeydown={theme_saturation_revert} 
                            onchange={theme_saturation_save} 
                            ~value={current_settings.theme.saturation.to_string()} 
                        />
                    </div>
                    <div class="setting-actions">
                        if should_show_undo!(theme.saturation, current_settings, initial_settings, 0.001) {
                            <span 
                                class="clickable" title="Undo"
                                onclick={theme_saturation_undo}
                            >{"↩️"}</span>
                        }
                        if should_show_reset!(theme.saturation, current_settings, settings_context, 0.001) {
                            <span 
                                class="clickable" title="Reset to default"
                                onclick={theme_saturation_reset}
                            >{"🔄"}</span>
                        }
                    </div>
                    <label for="theme_variant">{"Theme variant: "}</label>
                    <select 
                        id="theme_variant"
                        class={setting_class!(initial_settings, current_settings, theme.variant)} 
                        onchange={theme_variant_save}
                        ref={theme_variant_ref}
                    >
                        for name in ThemeVariant::VARIANTS {
                            <option key={*name}>{*name}</option>
                        }
                    </select>
                    <div class="setting-actions">
                        if should_show_undo!(theme.variant, current_settings, initial_settings) {
                            <span 
                                class="clickable" title="Undo"
                                onclick={theme_variant_undo}
                            >{"↩️"}</span>
                        }
                        if should_show_reset!(theme.variant, current_settings, settings_context) {
                            <span 
                                class="clickable" title="Reset to default"
                                onclick={theme_variant_reset}
                            >{"🔄"}</span>
                        }
                    </div>
                }
            </fieldset>
            <fieldset>
                <legend>{"Site behaviour"}</legend>
                <label for="enable_autosearch" title={AUTOSEARCH_TITLE}>{"Enable autosearch: "}</label>
                <input 
                    class={setting_class!(initial_settings, current_settings, enable_autosearch)} 
                    id="enable_autosearch" 
                    type="checkbox"
                    onchange={enable_autosearch_save} 
                    ~checked={current_settings.enable_autosearch} 
                />
                <div class="setting-actions">
                    if should_show_undo!(enable_autosearch, current_settings, initial_settings) {
                        <span 
                            class="clickable" title="Undo"
                            onclick={enable_autosearch_undo}
                        >{"↩️"}</span>
                    }
                    if should_show_reset!(enable_autosearch, current_settings, settings_context) {
                        <span 
                            class="clickable" title="Reset to default"
                            onclick={enable_autosearch_reset}
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
                    oninput={baseurl_oninput.clone()} 
                    onkeydown={thumbgen_api_base_url_revert} 
                    onchange={thumbgen_api_base_url_save} 
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
                <label for="disable_sharedworker" title={DISABLE_SW_TITLE}>{"Disable SharedWorker implementation: "}</label>
                <input 
                    class={setting_class!(initial_settings, current_settings, disable_sharedworker)} 
                    id="disable_sharedworker" 
                    title={DISABLE_SW_TITLE}
                    type="checkbox"
                    onchange={disable_sharedworker_save} 
                    ~checked={current_settings.disable_sharedworker} 
                />
                <div class="setting-actions">
                    if should_show_undo!(disable_sharedworker, current_settings, initial_settings) {
                        <span 
                            class="clickable" title="Undo"
                            onclick={disable_sharedworker_undo}
                        >{"↩️"}</span>
                    }
                    if should_show_reset!(disable_sharedworker, current_settings, settings_context) {
                        <span 
                            class="clickable" title="Reset to default"
                            onclick={disable_sharedworker_reset}
                        >{"🔄"}</span>
                    }
                </div>
            </fieldset>
            <fieldset>
                <legend>{"Credentials & authenticated actions"}</legend>
                <label for="private_user_id">{"Private userID: "}</label>
                <input 
                    class={setting_class!(initial_settings, current_settings, private_user_id)} 
                    id="private_user_id" 
                    type="password" minlength=30 
                    oninput={private_user_id_oninput} 
                    onkeydown={private_user_id_revert} 
                    onchange={private_user_id_save} 
                    oncopy={password_copy}
                    ~value={current_settings.private_user_id.to_string()} 
                />
                <div class="setting-actions">
                    if should_show_undo!(private_user_id, current_settings, initial_settings) {
                        <span 
                            class="clickable" title="Undo"
                            onclick={private_user_id_undo}
                        >{"↩️"}</span>
                    }
                    if should_show_reset!(private_user_id, current_settings, settings_context) {
                        <span 
                            class="clickable" title="Reset to default"
                            onclick={private_user_id_reset}
                        >{"🔄"}</span>
                    }
                </div>
                <label for="sponsorblock_api_base_url">{"SponsorBlock/DeArrow API base URL: "}</label>
                <input 
                    class={setting_class!(initial_settings, current_settings, sponsorblock_api_base_url)} 
                    id="sponsorblock_api_base_url" 
                    type="url" required=true
                    oninput={baseurl_oninput} 
                    onkeydown={sponsorblock_api_base_url_revert} 
                    onchange={sponsorblock_api_base_url_save} 
                    ~value={current_settings.sponsorblock_api_base_url.to_string()} 
                />
                <div class="setting-actions">
                    if should_show_undo!(sponsorblock_api_base_url, current_settings, initial_settings) {
                        <span 
                            class="clickable" title="Undo"
                            onclick={sponsorblock_api_base_url_undo}
                        >{"↩️"}</span>
                    }
                    if should_show_reset!(sponsorblock_api_base_url, current_settings, settings_context) {
                        <span 
                            class="clickable" title="Reset to default"
                            onclick={sponsorblock_api_base_url_reset}
                        >{"🔄"}</span>
                    }
                </div>
            </fieldset>
        </div>
    }
}
