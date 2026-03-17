use std::time::Duration;

use anyhow::Result;
use axum::extract::{DefaultBodyLimit, FromRef};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::normalize_path::NormalizePathLayer;

use crate::api::controllers;
use crate::api::state::*;

pub struct ApiEndpointBuilder<C = ()> {
    common: ApiEndpointBuilderCommon,
    #[allow(unused)]
    custom_routes: C,
}

impl Default for ApiEndpointBuilder {
    #[inline]
    fn default() -> Self {
        Self {
            common: Default::default(),
            custom_routes: (),
        }
    }
}

impl ApiEndpointBuilder<()> {
    #[allow(unused)]
    pub fn with_custom_routes<S>(
        self,
        routes: axum::Router<S>,
    ) -> ApiEndpointBuilder<axum::Router<S>>
    where
        ApiState: FromRef<S>,
        S: Send + Sync,
    {
        ApiEndpointBuilder {
            common: self.common,
            custom_routes: routes,
        }
    }

    pub async fn bind(self, state: ApiState) -> Result<ApiEndpoint> {
        let listener = state.bind_socket().await?;
        let origins = state.api_config().cors_origins.clone();
        Ok(ApiEndpoint::from_parts(
            listener,
            self.common.build(),
            state,
            &origins,
        ))
    }
}

struct ApiEndpointBuilderCommon {
    healthcheck_route: Option<String>,
}

impl Default for ApiEndpointBuilderCommon {
    fn default() -> Self {
        Self {
            healthcheck_route: Some("/".to_owned()),
        }
    }
}

impl ApiEndpointBuilderCommon {
    fn build<S>(self) -> axum::Router<S>
    where
        ApiState: FromRef<S>,
        S: Clone + Send + Sync + 'static,
    {
        let mut router = axum::Router::new();

        if let Some(route) = self.healthcheck_route {
            router = router.route(&route, get(health_check));
        }

        router
            .nest("/auth", auth_router())
            .nest("/user", user_router())
    }
}

pub struct ApiEndpoint {
    listener: TcpListener,
    router: axum::Router<()>,
}

impl ApiEndpoint {
    pub fn builder() -> ApiEndpointBuilder {
        ApiEndpointBuilder::default()
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
        use tower::ServiceBuilder;
        use tower_http::timeout::TimeoutLayer;

        let service = ServiceBuilder::new()
            .layer(DefaultBodyLimit::max(MAX_REQUEST_SIZE))
            .layer(NormalizePathLayer::trim_trailing_slash())
            .layer(build_cors(origins))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_secs(25),
            ));

        #[cfg(feature = "compression")]
        let service = service.layer(tower_http::compression::CompressionLayer::new().gzip(true));

        // Prepare routes
        let router = router.layer(service).with_state(state);

        // Done
        Self { listener, router }
    }

    pub async fn serve(self, token: CancellationToken) -> std::io::Result<()> {
        axum::serve(self.listener, self.router)
            .with_graceful_shutdown(async move { token.cancelled().await })
            .await
    }
}

fn auth_router<S>() -> axum::Router<S>
where
    ApiState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    axum::Router::new()
        .route("/", get(controllers::auth::login))
        .route("/callback", get(controllers::auth::callback))
        .route("/logout", post(controllers::auth::logout))
        .route("/public-key", get(controllers::auth::public_key))
}

fn user_router<S>() -> axum::Router<S>
where
    ApiState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    axum::Router::new()
        .route("/info", get(controllers::user::info))
        .route(
            "/devices",
            get(controllers::device::list).post(controllers::device::bind),
        )
}

fn health_check() -> futures_util::future::Ready<impl IntoResponse> {
    futures_util::future::ready(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before Unix epoch")
            .as_millis()
            .to_string(),
    )
}

const MAX_REQUEST_SIZE: usize = 2 << 17; // 256kb

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
