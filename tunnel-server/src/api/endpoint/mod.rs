use anyhow::Result;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::api::controllers;
use crate::state::TunnelState;

// ── Builder ───────────────────────────────────────────────────────────────

pub struct TunnelEndpointBuilder {
    healthcheck_route: Option<String>,
}

impl Default for TunnelEndpointBuilder {
    fn default() -> Self {
        Self {
            healthcheck_route: Some("/health".to_owned()),
        }
    }
}

impl TunnelEndpointBuilder {
    pub async fn bind(self, state: TunnelState) -> Result<TunnelEndpoint> {
        let listener = state.bind_socket().await?;
        let origins = state.api_config().clone().cors_origins;
        Ok(TunnelEndpoint::from_parts(
            listener,
            self.build_router(),
            state,
            &origins,
        ))
    }

    fn build_router<S>(&self) -> axum::Router<S>
    where
        TunnelState: axum::extract::FromRef<S>,
        S: Clone + Send + Sync + 'static,
    {
        let mut router = axum::Router::new();

        if let Some(route) = &self.healthcheck_route {
            router = router.route(route, get(health_check));
        }

        router
            .route(
                "/tunnel/{device_id}/session",
                post(controllers::tunnel::create_session),
            )
            .route("/device/connect", get(controllers::device::connect))
            .fallback(controllers::tunnel::proxy)
    }
}

// ── Endpoint ──────────────────────────────────────────────────────────────

pub struct TunnelEndpoint {
    listener: TcpListener,
    router: axum::Router<()>,
}

impl TunnelEndpoint {
    pub fn builder() -> TunnelEndpointBuilder {
        TunnelEndpointBuilder::default()
    }

    pub fn from_parts<S>(
        listener: TcpListener,
        router: axum::Router<S>,
        state: S,
        origins: &[String],
    ) -> Self
    where
        S: Clone + Send + Sync + 'static,
    {
        let service = ServiceBuilder::new().layer(build_cors(origins));

        let router = router.layer(service).with_state(state);

        Self { listener, router }
    }

    pub async fn serve(self, token: CancellationToken) -> std::io::Result<()> {
        axum::serve(self.listener, self.router)
            .with_graceful_shutdown(async move { token.cancelled().await })
            .await
    }
}

async fn health_check() -> impl IntoResponse {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_millis()
        .to_string()
}

fn build_cors(origins: &[String]) -> CorsLayer {
    if origins.is_empty() {
        return CorsLayer::permissive();
    }

    let origins: Vec<axum::http::HeaderValue> = origins
        .iter()
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
}
