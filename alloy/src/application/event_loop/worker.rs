//! Background fetch threads for navigations, stylesheets, and images.

use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;

use graphics::{Framebuffer, ImageId};
use network::{HttpRequest, HttpTransport, RequestPolicy, Url};

use super::session::LoopMessage;
use crate::application::navigation;
use crate::error::AlloyError;

pub fn spawn_navigation(
    url: Url,
    transport: Arc<dyn HttpTransport>,
    policy: Arc<dyn RequestPolicy>,
    sender: Sender<LoopMessage>,
) {
    thread::spawn(move || {
        let result = navigation::navigate(&url, transport.as_ref(), policy.as_ref())
            .map(|dom_tree| (dom_tree, url));
        let _ = sender.send(LoopMessage::Navigation(result));
    });
}

pub fn spawn_stylesheet_fetch(
    url: Url,
    transport: Arc<dyn HttpTransport>,
    sender: Sender<LoopMessage>,
) {
    thread::spawn(move || {
        let result = fetch_text(&url, transport.as_ref());
        let _ = sender.send(LoopMessage::Stylesheet(result));
    });
}

pub fn spawn_image_fetch(
    id: ImageId,
    url: Url,
    transport: Arc<dyn HttpTransport>,
    sender: Sender<LoopMessage>,
) {
    thread::spawn(move || {
        let result = fetch_image(&url, transport.as_ref());
        let _ = sender.send(LoopMessage::Image(id, result));
    });
}

pub fn fetch_text(url: &Url, transport: &dyn HttpTransport) -> Result<String, AlloyError> {
    let response = transport.execute(&HttpRequest::get(url.clone()))?;
    navigation::ensure_success(url, response.status())?;
    let body = response.body().as_str().unwrap_or_default().to_owned();
    tracing::debug!(
        %url,
        status = response.status().code(),
        bytes = body.len(),
        "stylesheet fetched"
    );
    Ok(body)
}

pub fn fetch_image(url: &Url, transport: &dyn HttpTransport) -> Result<Framebuffer, AlloyError> {
    let response = transport.execute(&HttpRequest::get(url.clone()))?;
    navigation::ensure_success(url, response.status())?;
    Ok(graphics::png::decode(response.body().as_bytes())?)
}
