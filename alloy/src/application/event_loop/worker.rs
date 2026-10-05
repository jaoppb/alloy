//! Background fetch threads for navigations, stylesheets, and images.

use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;

use dom::DomTree;
use graphics::{Framebuffer, ImageId};
use network::{HttpRequest, HttpTransport, RequestPolicy, Url};

use super::generation::NavigationGeneration;
use crate::application::navigation;
use crate::application::subresource::SubresourceRequest;
use crate::error::AlloyError;

/// What a background fetch produced, drained by the loop's own thread.
///
/// Every variant leads with the [`NavigationGeneration`] its worker was
/// started under, so the session can drop a result from a superseded
/// navigation.
pub enum LoopMessage {
    Navigation(NavigationGeneration, Result<(DomTree, Url), AlloyError>),
    Stylesheet(NavigationGeneration, Result<String, AlloyError>),
    Image(
        NavigationGeneration,
        ImageId,
        Result<Framebuffer, AlloyError>,
    ),
}

impl LoopMessage {
    pub const fn generation(&self) -> NavigationGeneration {
        match self {
            Self::Navigation(generation, _)
            | Self::Stylesheet(generation, _)
            | Self::Image(generation, ..) => *generation,
        }
    }
}

pub fn spawn_navigation<T, P>(
    url: Url,
    transport: Arc<T>,
    policy: Arc<P>,
    generation: NavigationGeneration,
    sender: Sender<LoopMessage>,
) where
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
{
    thread::spawn(move || {
        let result = navigation::navigate(&url, transport.as_ref(), policy.as_ref())
            .map(|dom_tree| (dom_tree, url));
        let _ = sender.send(LoopMessage::Navigation(generation, result));
    });
}

pub fn spawn_subresource_fetch<T: HttpTransport + 'static>(
    request: SubresourceRequest,
    transport: Arc<T>,
    generation: NavigationGeneration,
    sender: &Sender<LoopMessage>,
) {
    let sender = sender.clone();
    thread::spawn(move || {
        let _ = sender.send(fetch_subresource(request, transport.as_ref(), generation));
    });
}

/// The one place a [`SubresourceRequest`] variant maps to how it is fetched.
fn fetch_subresource<T: HttpTransport>(
    request: SubresourceRequest,
    transport: &T,
    generation: NavigationGeneration,
) -> LoopMessage {
    match request {
        SubresourceRequest::Stylesheet(stylesheet) => {
            LoopMessage::Stylesheet(generation, fetch_text(stylesheet.url(), transport))
        }
        SubresourceRequest::Image(image) => {
            LoopMessage::Image(generation, image.id(), fetch_image(image.url(), transport))
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
