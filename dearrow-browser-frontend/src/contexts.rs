use std::rc::Rc;

use dearrow_browser_api::StatusResponse;
use reqwest::Url;
use yew::AttrValue;

pub use crate::components::modal_renderer::ModalRendererControls;

#[derive(Clone, PartialEq)]
pub struct WindowContext {
    pub origin: Url,
    pub logo_url: Option<AttrValue>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct UpdateClock(pub bool);

pub type StatusContext = Option<Rc<StatusResponse>>;
