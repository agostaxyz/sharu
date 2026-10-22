// SPDX-License-Identifier: AGPL-3.0-or-later
mod codes;
mod config;
mod hub;
mod nick;
mod ratelimit;
mod ws;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::header::{
    CONTENT_SECURITY_POLICY, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderName, HeaderValue};
use axum::routing::get;
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tracing_subscriber::EnvFilter;

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; \
img-src 'self' data: blob:; media-src 'self' blob:; connect-src 'self' ws: wss:; \
object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = config::Config::from_env();
    tracing::info!(
        words = codes::words().len(),
        entropy_bits = format!("{:.1}", codes::entropy_bits(cfg.hub.code_words)),
        "session code space"
    );
    let (bind, static_dir) = (cfg.bind, cfg.static_dir.clone());
    let app = Arc::new(ws::App::new(cfg));

    // No request-logging layer on purpose: URLs contain session codes.
    let mut router = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/ws", get(ws::ws_handler))
        .with_state(app);
    if let Some(dir) = static_dir {
        let fallback = ServeFile::new(dir.join("404.html"));
        router = router.fallback_service(ServeDir::new(&dir).fallback(fallback));
    }
    let router = router
        .layer(SetResponseHeaderLayer::if_not_present(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CSP),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static(
                "display-capture=(self), camera=(), microphone=(), geolocation=()",
            ),
        ));

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .expect("cannot bind BIND address");
    tracing::info!(%bind, "sharu-server listening");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
    .expect("server error");
}
