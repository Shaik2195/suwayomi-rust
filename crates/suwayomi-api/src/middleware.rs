use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::prelude::*;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

pub fn setup_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::COOKIE,
        ])
        .allow_origin(Any)
}

pub fn setup_tracing(
) -> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>>
{
    TraceLayer::new_for_http()
}

pub fn setup_compression() -> CompressionLayer {
    CompressionLayer::new()
}

pub fn get_auth_credentials() -> Option<(String, String)> {
    let user = std::env::var("BASIC_AUTH_USERNAME")
        .or_else(|_| std::env::var("SUWAYOMI_BASIC_AUTH_USERNAME"))
        .unwrap_or_default();
    let pass = std::env::var("BASIC_AUTH_PASSWORD")
        .or_else(|_| std::env::var("SUWAYOMI_BASIC_AUTH_PASSWORD"))
        .unwrap_or_default();
    if !user.trim().is_empty() && !pass.trim().is_empty() {
        Some((user, pass))
    } else {
        None
    }
}

pub async fn basic_auth_middleware(req: Request, next: Next) -> Response {
    basic_auth_impl(req, next, get_auth_credentials()).await
}

pub async fn basic_auth_impl(
    req: Request,
    next: Next,
    creds: Option<(String, String)>,
) -> Response {
    let Some((expected_user, expected_pass)) = creds else {
        return next.run(req).await;
    };

    let path = req.uri().path();
    // Exempt public static branding and health check endpoints
    if path == "/site.webmanifest"
        || path == "/favicon.ico"
        || path == "/favicon.svg"
        || path == "/favicon-96x96.png"
        || path == "/apple-touch-icon.png"
        || path == "/health"
        || path == "/api/health"
    {
        return next.run(req).await;
    }

    let expected_token = BASE64_STANDARD.encode(format!("{}:{}", expected_user, expected_pass));

    // 1. Check cached session cookie: suwayomi_session=<token>
    if let Some(cookie_header) = req.headers().get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        for cookie in cookie_header.split(';') {
            if let Some((name, val)) = cookie.trim().split_once('=') {
                if name.trim() == "suwayomi_session" && val.trim() == expected_token {
                    return next.run(req).await;
                }
            }
        }
    }

    // 2. Check Authorization: Basic <base64>
    let mut has_valid_basic = false;
    if let Some(auth_header) = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(base64_str) = auth_header.strip_prefix("Basic ").map(str::trim) {
            if let Ok(decoded_bytes) = BASE64_STANDARD.decode(base64_str) {
                if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                    if let Some((user, pass)) = decoded_str.split_once(':') {
                        if user == expected_user && pass == expected_pass {
                            has_valid_basic = true;
                        }
                    }
                }
            }
        }
    }

    if has_valid_basic {
        let mut response = next.run(req).await;
        // Cache session via HttpOnly Cookie for 30 days so user isn't re-prompted
        if let Ok(cookie_val) = HeaderValue::from_str(&format!(
            "suwayomi_session={}; Path=/; Max-Age=2592000; SameSite=Lax; HttpOnly",
            expected_token
        )) {
            response.headers_mut().insert(header::SET_COOKIE, cookie_val);
        }
        return response;
    }

    // 3. Return 401 Unauthorized with standard Basic Auth challenge
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(
            header::WWW_AUTHENTICATE,
            HeaderValue::from_static("Basic realm=\"Suwayomi\", charset=\"UTF-8\""),
        )
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )
        .body(Body::from("401 Unauthorized: Basic credentials required"))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn create_test_app(creds: Option<(String, String)>) -> Router {
        Router::new()
            .route("/test", get(|| async { "ok" }))
            .route("/health", get(|| async { "healthy" }))
            .layer(axum::middleware::from_fn(move |req, next| {
                let creds = creds.clone();
                async move { basic_auth_impl(req, next, creds).await }
            }))
    }

    #[tokio::test]
    async fn test_basic_auth_disabled_when_none() {
        let app = create_test_app(None);
        let req = axum::http::Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn test_basic_auth_unauthorized_when_no_credentials() {
        let creds = Some(("admin".to_string(), "secret123".to_string()));
        let app = create_test_app(creds);
        let req = axum::http::Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            resp.headers().get(header::WWW_AUTHENTICATE).unwrap(),
            "Basic realm=\"Suwayomi\", charset=\"UTF-8\""
        );
    }

    #[tokio::test]
    async fn test_basic_auth_unauthorized_when_wrong_credentials() {
        let creds = Some(("admin".to_string(), "secret123".to_string()));
        let app = create_test_app(creds);
        let bad_auth = BASE64_STANDARD.encode("admin:wrongpass");
        let req = axum::http::Request::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, format!("Basic {}", bad_auth))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_basic_auth_authorized_and_sets_session_cookie() {
        let creds = Some(("admin".to_string(), "secret123".to_string()));
        let app = create_test_app(creds);
        let good_auth = BASE64_STANDARD.encode("admin:secret123");
        let req = axum::http::Request::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, format!("Basic {}", good_auth))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .unwrap();
        assert!(cookie.contains(&format!("suwayomi_session={}", good_auth)));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Max-Age=2592000"));
    }

    #[tokio::test]
    async fn test_basic_auth_cached_session_cookie_bypasses_credentials() {
        let creds = Some(("admin".to_string(), "secret123".to_string()));
        let app = create_test_app(creds);
        let token = BASE64_STANDARD.encode("admin:secret123");
        let req = axum::http::Request::builder()
            .uri("/test")
            .header(header::COOKIE, format!("suwayomi_session={}", token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_basic_auth_exempts_health_endpoint() {
        let creds = Some(("admin".to_string(), "secret123".to_string()));
        let app = create_test_app(creds);
        let req = axum::http::Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
