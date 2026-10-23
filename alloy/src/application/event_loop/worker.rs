//! Background fetch threads: the executor side of [`BrowserEffect::Fetch`].
//!
//! `core/browser` only says *what* to fetch; this module runs it on a
//! `std::thread` and sends the stamped [`FetchResult`] back to the loop's own
//! thread. A thread cannot be cancelled — a superseded navigation's result
//! still arrives and is dropped by its [`NavigationId`], not here.
//!
//! [`BrowserEffect::Fetch`]: browser::BrowserEffect::Fetch

use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;

use browser::{
    FetchFailure, FetchPayload, FetchRequest, FetchResult, LoadedDocument, NavigationId,
    SubresourceRequest,
};
use graphics::Framebuffer;
use network::{HttpRequest, HttpTransport, RequestPolicy, Url};

use crate::application::navigation;
use crate::error::AlloyError;

/// Runs `request` on its own thread and reports the outcome, stamped with
/// `navigation`, on `sender`.
pub fn spawn_fetch<T, P>(
    request: FetchRequest,
    navigation: NavigationId,
    transport: Arc<T>,
    policy: Arc<P>,
    sender: Sender<FetchResult>,
) where
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
{
    thread::spawn(move || {
        let payload = fetch(request, transport.as_ref(), policy.as_ref());
        let _ = sender.send(FetchResult::new(navigation, payload));
    });
}

/// The one place a [`FetchRequest`] variant maps to how it is fetched.
fn fetch<T: HttpTransport, P: RequestPolicy>(
    request: FetchRequest,
    transport: &T,
    policy: &P,
) -> FetchPayload {
    match request {
        FetchRequest::Document(url) => {
            FetchPayload::Document(fetch_document(url, transport, policy))
        }
        FetchRequest::Subresource(SubresourceRequest::Stylesheet(stylesheet)) => {
            let media = stylesheet.media().clone();
            let text = fetch_text(stylesheet.url(), transport).map_err(|error| describe(&error));
            FetchPayload::Stylesheet(media, text)
        }
        FetchRequest::Subresource(SubresourceRequest::Image(image)) => {
            let frame = fetch_image(image.url(), transport).map_err(|error| describe(&error));
            FetchPayload::Image(image.id(), frame)
        }
    }
}

fn fetch_document<T: HttpTransport, P: RequestPolicy>(
    url: Url,
    transport: &T,
    policy: &P,
) -> Result<LoadedDocument, FetchFailure> {
    let tree = navigation::navigate(&url, transport, policy).map_err(|error| describe(&error))?;
    Ok(LoadedDocument::new(tree, url))
}

/// The error page shows the message; the cause stays in this crate's error.
fn describe(error: &AlloyError) -> FetchFailure {
    FetchFailure::new(error.to_string())
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
