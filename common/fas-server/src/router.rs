use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderName, HeaderValue, StatusCode, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use tower::ServiceBuilder;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;

use crate::controllers::{BinauthController, FasController, HealthController, StatusController};
use crate::peer_ip::PeerIp;
use crate::{AppState, RateLimiter, RouterLimits};

const BINAUTH_BODY_LIMIT: usize = 4096;

async fn limit_fas_requests(
    State(limiter): State<RateLimiter>,
    PeerIp(peer): PeerIp,
    request: Request,
    next: Next,
) -> Response {
    let client = peer.unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
    if limiter.check(client) {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, "60")],
            "too many requests",
        )
            .into_response()
    }
}

pub fn public_router(state: AppState, limits: RouterLimits) -> Router {
    let limiter = RateLimiter::per_minute(limits.fas_requests_per_minute);
    let fas = Router::new()
        .route("/fas", get(FasController::auth))
        .route_layer(from_fn_with_state(limiter, limit_fas_requests));

    let hardening = ServiceBuilder::new()
        .layer(CatchPanicLayer::new())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            limits.request_timeout,
        ))
        .layer(ConcurrencyLimitLayer::new(limits.max_concurrent_requests))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ));

    Router::new()
        .merge(fas)
        .route("/health", get(HealthController::health))
        .with_state(state)
        .layer(hardening)
}

pub fn admin_router(state: AppState) -> Router {
    Router::new()
        .route("/status", get(StatusController::status))
        .route(
            "/binauth",
            post(BinauthController::event)
                .layer(axum::extract::DefaultBodyLimit::max(BINAUTH_BODY_LIMIT)),
        )
        .with_state(state)
}
