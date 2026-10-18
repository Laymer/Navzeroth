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
    response(
        StatusCode::from_u16(reply.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        reply.body.to_vec(),
        reply.content_type,
    )
}

pub fn respond_stored(
    request: &Request<Vec<u8>>,
    host: &azeroth_app::recovery::DatabaseHost,
) -> Response<Vec<u8>> {
    respond_private(request, |path| {
        host.with_application(|app| app.stored_atlas_asset(path))
            .ok()
            .flatten()
    })
}

pub fn respond_research(
    request: &Request<Vec<u8>>,
    service: &azeroth_app::local_data::LocalDataService,
) -> Response<Vec<u8>> {
    respond_private(request, |path| service.research_asset("GET", path, false))
}

// Policy applies before consulting a verified asset provider. Keeping it shared
// also prevents namespace adapters from accidentally accepting query aliases.
fn respond_private(
    request: &Request<Vec<u8>>,
    asset: impl FnOnce(&str) -> Option<Vec<u8>>,
) -> Response<Vec<u8>> {
    let body = if request.method().as_str() == "GET" && request.uri().query().is_none() {
        asset(request.uri().path())
    } else {
        None
    };
    let status = if request.method().as_str() != "GET" {
        StatusCode::METHOD_NOT_ALLOWED
    } else if body.is_some() {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    };
    response(
        status,
        body.unwrap_or_default(),
        (status == StatusCode::OK).then_some("image/png"),
    )
}

fn response(
    status: StatusCode,
    body: Vec<u8>,
    content_type: Option<&'static str>,
) -> Response<Vec<u8>> {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    if let Some(content_type) = content_type {
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(method: &str, path: &str) -> Request<Vec<u8>> {
        Request::builder()
            .method(method)
            .uri(path)
            .body(vec![])
            .unwrap()
    }
    fn assert_headers(reply: &Response<Vec<u8>>) {
        assert_eq!(reply.headers()[CACHE_CONTROL], "no-store");
        assert_eq!(reply.headers()[X_CONTENT_TYPE_OPTIONS], "nosniff");
    }
    #[test]
    fn private_transport_refuses_query_aliases_before_consulting_a_verified_provider() {
        let png = serve_atlas("GET", "/demo-image-1/overview").body.to_vec();
        for namespace in ["research", "stored"] {
            let path = format!("/{namespace}/{}/{}", "a".repeat(64), "b".repeat(64));
            let positive = respond_private(&request("GET", &path), |selected| {
                assert_eq!(selected, path);
                Some(png.clone())
            });
            assert_eq!(positive.status(), StatusCode::OK);
            assert_eq!(positive.body(), &png);
            assert_eq!(positive.headers()[CONTENT_TYPE], "image/png");
            assert_headers(&positive);
            for (method, selected, status) in [
                (
                    "GET",
                    format!("{path}?substitute=true"),
                    StatusCode::NOT_FOUND,
                ),
                ("POST", path.clone(), StatusCode::METHOD_NOT_ALLOWED),
            ] {
                let refused = respond_private(&request(method, &selected), |_| {
                    panic!("Refused aliases must never consult the asset provider")
                });
                assert_eq!(refused.status(), status);
                assert!(refused.body().is_empty());
                assert_headers(&refused);
                assert!(!refused.headers().contains_key(CONTENT_TYPE));
            }
        }
    }
    #[test]
    fn native_scheme_preserves_checked_png_and_empty_refusals() {
        let ok = respond(&request("GET", "/demo-image-1/overview"));
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(ok.headers()[CONTENT_TYPE], "image/png");
        assert!(!ok.body().is_empty());
        assert_headers(&ok);
        for (method, path, status) in [
            (
                "POST",
                "/demo-image-1/overview",
                StatusCode::METHOD_NOT_ALLOWED,
            ),
            ("GET", "/unknown/overview", StatusCode::NOT_FOUND),
        ] {
            let refused = respond(&request(method, path));
            assert_eq!(refused.status(), status);
            assert!(refused.body().is_empty());
            assert!(!refused.headers().contains_key(CONTENT_TYPE));
            assert_headers(&refused);
        }
    }
    #[test]
    fn private_schemes_reject_methods_and_unbound_assets_without_cacheable_errors() {
        let root = std::env::temp_dir().join(format!("nav-native-atlas-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let host = azeroth_app::recovery::DatabaseHost::open(&root.join("state.sqlite3"));
        let local = azeroth_app::local_data::LocalDataService::new(root.join("private"));
        for (method, path, status) in [
            ("POST", "/stored/unknown", StatusCode::METHOD_NOT_ALLOWED),
            (
                "GET",
                "/stored/unknown?substitute=true",
                StatusCode::NOT_FOUND,
            ),
            ("GET", "/stored/unknown", StatusCode::NOT_FOUND),
        ] {
            let input = request(method, path);
            for reply in [
                respond_stored(&input, &host),
                respond_research(&input, &local),
            ] {
                assert_eq!(reply.status(), status);
                assert_headers(&reply);
                assert!(reply.body().is_empty());
                assert!(!reply.headers().contains_key(CONTENT_TYPE));
            }
        }
        drop((host, local));
        std::fs::remove_dir_all(root).unwrap();
    }
}
