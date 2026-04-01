/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2026 mini_bomba
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

use chrono::{Datelike, Local};
use gloo_console::error;
use wasm_bindgen::JsCast;
use web_sys::{CssStyleRule, CssStyleSheet, Document, Element};
use yew::{Component, ContextHandle, html};

use crate::{contexts::settings::SettingsContext, settings::{Theme, ThemeVariant}};

pub(super) struct ThemeRenderer {
    document: Document,
    document_element: Element,
    stylesheet: CssStyleSheet,
    rule: CssStyleRule,
    theme: Theme,

    _settings_listener: ContextHandle<SettingsContext>,
}

macro_rules! js_expect {
    ($ctx:literal) => {
        |err| {
            error!($ctx, err);
            panic!($ctx);
        }
    };
}

impl ThemeRenderer {
    fn update_theme(&self, was_enabled: bool) {
        if was_enabled && !self.theme.enable {
            // remove self
            let sheets = self.document.adopted_style_sheets();
            let self_idx = sheets.index_of(&self.stylesheet, 0);
            if self_idx >= 0 {
                sheets.delete(self_idx.cast_unsigned());
            }
        }
        let style = self.rule.style();
        if let Err(e) = style.set_property("--theme-hue", &format!("{:.3}", self.theme.hue)) {
            error!("Failed to update theme hue: ", e);
        }
        if let Err(e) = style.set_property("--theme-saturation", &format!("{:.3}%", self.theme.saturation)) {
            error!("Failed to update theme saturation: ", e);
        }

        let light_enabled = self.document_element.has_attribute("data-theme-light");
        let light_set = self.theme.enable && self.theme.variant == ThemeVariant::Light;

        if light_enabled && !light_set {
            if let Err(e) = self.document_element.remove_attribute("data-theme-light") {
                error!("Failed to disable light mode: ", e);
            }
        }
        if !light_enabled && light_set {
            if let Err(e) = self.document_element.set_attribute("data-theme-light", "1") {
                error!("Failed to enable light mode: ", e);
            }
        }

        if !was_enabled && self.theme.enable {
            // add self
            let sheets = self.document.adopted_style_sheets();
            sheets.push(&self.stylesheet);
        }
    }

    fn april_fools(new_theme: Theme) -> Theme {
        if new_theme.enable {
            return new_theme;
        }
        let ts = Local::now();
        if ts.month() == 4 && ts.day() == 1 {
            return Theme {
                enable: true,
                variant: ThemeVariant::Light,
                hue: 0.,
                saturation: 95.,
            }
        }
        new_theme
    }
}

impl Component for ThemeRenderer {
    type Properties = ();
    type Message = Theme;

    fn create(ctx: &yew::Context<Self>) -> Self {
        let scope = ctx.link();
        let (settings, settings_listener) = scope
            .context(scope.callback(|settings: SettingsContext| settings.settings().theme))
            .expect("settings context should exist");

        let window = web_sys::window().expect("window should exist");
        let document = window.document().expect("document should exist");
        let document_element = document.document_element().expect("document should have its element");

        let stylesheet =
            CssStyleSheet::new().unwrap_or_else(js_expect!("Failed to construct a CSSStyleSheet"));
        let rule_idx = stylesheet
            .insert_rule(":root {}")
            .unwrap_or_else(js_expect!("Failed to insert the :root CSS rule"));
        let rule = stylesheet
            .css_rules()
            .unwrap_or_else(js_expect!("Failed to get CSSStyleSheet's .cssRules"))
            .item(rule_idx)
            .expect("Failed to retrieve the rule we just inserted")
            .dyn_into::<CssStyleRule>()
            .expect("The CSSStyleRule we just inserted wasn't actually a CSSStyleRule??????");

        let this = Self {
            document,
            document_element,
            stylesheet,
            rule,
            theme: Self::april_fools(settings.settings().theme),

            _settings_listener: settings_listener,
        };
        this.update_theme(false);
        this
    }

    fn view(&self, _ctx: &yew::Context<Self>) -> yew::Html {
        html! {}
    }

    fn update(&mut self, _ctx: &yew::Context<Self>, msg: Self::Message) -> bool {
        let new_theme = Self::april_fools(msg);
        if new_theme != self.theme {
            let was_enabled = self.theme.enable;
            self.theme = new_theme;
            self.update_theme(was_enabled);
        }
        false
    }
}
