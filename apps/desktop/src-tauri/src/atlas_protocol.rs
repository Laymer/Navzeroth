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

pub fn respond_stored(
    request: &Request<Vec<u8>>,
    host: &azeroth_app::recovery::DatabaseHost,
) -> Response<Vec<u8>> {
    let outcome = if request.method().as_str() != "GET" || request.uri().query().is_some() {
        None
    } else {
        host.with_application(|app| app.stored_atlas_asset(request.uri().path()))
            .ok()
            .flatten()
    };
    let status = if request.method().as_str() != "GET" {
        405
    } else if outcome.is_some() {
        200
    } else {
        404
    };
    let mut response = Response::new(outcome.unwrap_or_default());
    *response.status_mut() = StatusCode::from_u16(status).unwrap_or(StatusCode::NOT_FOUND);
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    if status == 200 {
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    }
    response
}

pub fn respond_research(
    request: &Request<Vec<u8>>,
    service: &azeroth_app::local_data::LocalDataService,
) -> Response<Vec<u8>> {
    let body = service.research_asset(
        request.method().as_str(),
        request.uri().path(),
        request.uri().query().is_some(),
    );
    let status = if request.method().as_str() != "GET" {
        StatusCode::METHOD_NOT_ALLOWED
    } else if body.is_some() {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    };
    let mut response = Response::new(body.unwrap_or_default());
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    if status == StatusCode::OK {
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    }
    response
}
