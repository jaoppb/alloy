//! Background fetch threads for navigations, stylesheets, and images.

use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;

use dom::DomTree;
use graphics::{Framebuffer, ImageId};
use network::{HttpRequest, HttpTransport, RequestPolicy, Url};

use crate::application::navigation;
use crate::application::subresource::SubresourceRequest;
use crate::error::AlloyError;

/// What a background fetch produced, drained by the loop's own thread.
pub enum LoopMessage {
    Navigation(Result<(DomTree, Url), AlloyError>),
    Stylesheet(Result<String, AlloyError>),
    Image(ImageId, Result<Framebuffer, AlloyError>),
}

pub fn spawn_navigation<T, P>(
    url: Url,
    transport: Arc<T>,
    policy: Arc<P>,
    sender: Sender<LoopMessage>,
) where
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
{
    thread::spawn(move || {
        let result = navigation::navigate(&url, transport.as_ref(), policy.as_ref())
            .map(|dom_tree| (dom_tree, url));
        let _ = sender.send(LoopMessage::Navigation(result));
    });
}

pub fn spawn_subresource_fetch<T: HttpTransport + 'static>(
    request: SubresourceRequest,
    transport: Arc<T>,
    sender: &Sender<LoopMessage>,
) {
    let sender = sender.clone();
    thread::spawn(move || {
        let _ = sender.send(fetch_subresource(request, transport.as_ref()));
    });
}

/// The one place a [`SubresourceRequest`] variant maps to how it is fetched.
fn fetch_subresource<T: HttpTransport>(request: SubresourceRequest, transport: &T) -> LoopMessage {
    match request {
        SubresourceRequest::Stylesheet(stylesheet) => {
            LoopMessage::Stylesheet(fetch_text(stylesheet.url(), transport))
        }
        SubresourceRequest::Image(image) => {
            LoopMessage::Image(image.id(), fetch_image(image.url(), transport))
        }
    }
}

pub fn fetch_text<T: HttpTransport>(url: &Url, transport: &T) -> Result<String, AlloyError> {
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

fn fetch_image<T: HttpTransport>(url: &Url, transport: &T) -> Result<Framebuffer, AlloyError> {
    let response = transport.execute(&HttpRequest::get(url.clone()))?;
    navigation::ensure_success(url, response.status())?;
    Ok(graphics::png::decode(response.body().as_bytes())?)
}
