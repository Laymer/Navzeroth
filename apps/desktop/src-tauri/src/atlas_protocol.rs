//! Thin adapter from the pure `azeroth_app::atlas::serve_atlas` to the `atlas`
//! URI scheme. All decisions (path validation, dataset-declared layers, hash
//! verification, status codes) live in `azeroth-app` and are tested on Linux.
use azeroth_app::atlas::serve_atlas;
use tauri::http::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderValue, X_CONTENT_TYPE_OPTIONS};
use tauri::http::{Request, Response, StatusCode};

/// Scheme name registered on the builder; the webview reaches it at
/// `atlas://localhost/<image>/<layer>` (Linux, macOS) or
/// `http://atlas.localhost/<image>/<layer>` (Windows, Android).
pub const SCHEME: &str = "atlas";

pub fn respond(request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let reply = serve_atlas(request.method().as_str(), request.uri().path());
    let mut response = Response::new(reply.body.to_vec());
    *response.status_mut() =
        StatusCode::from_u16(reply.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let headers = response.headers_mut();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    if let Some(content_type) = reply.content_type {
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    }
    response
}
