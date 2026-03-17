use deadpool_postgres::{Config, Runtime};
use std::net::SocketAddr;
use tokio_postgres::NoTls;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use validator::Validate;

mod api;
use api::*;

mod middleware;
mod state;
use state::{Config as AppConfig, State};

mod handlers;

mod auth;
mod auth_helpers;
mod db;
mod models;

#[derive(Clone)]
struct ApiImpl {
    state: State,
}

impl ApiImpl {
    fn new() -> anyhow::Result<Self> {
        let host = std::env::var("POSTGRES_HOST").unwrap_or("localhost".to_string());
        let port = std::env::var("POSTGRES_PORT")
            .unwrap_or("5432".to_string())
            .parse::<u16>()?;
        let user = std::env::var("POSTGRES_USER").expect("POSTGRES_USER must be set");
        let password = std::env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD must be set");
        let dbname = std::env::var("POSTGRES_DB").expect("POSTGRES_DB must be set");

        let order_rate_limit_minutes = std::env::var("ORDER_RATE_LIMIT_MINUTES")
            .unwrap_or("5".to_string())
            .parse::<i64>()?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "super-secret-jwt-key-change-in-production".to_string());

        let mut cfg = Config::new();
        cfg.host = Some(host);
        cfg.port = Some(port);
        cfg.user = Some(user);
        cfg.password = Some(password);
        cfg.dbname = Some(dbname);

        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

        Ok(Self {
            state: State {
                db: pool,
                config: AppConfig {
                    order_rate_limit_minutes,
                    jwt_secret,
                },
            },
        })
    }
}

macro_rules! validate {
    ( $request:expr, $type:ty ) => {
        if let Err(e) = $request.validate() {
            return Ok(<$type>::BadRequest(ErrorResponse {
                error_code: ErrorResponseErrorCode::ValidationError,
                message: format!("Validation error: {}", e),
                details: None,
            }));
        }
    };
}

macro_rules! auth {
    ( $request:expr, $state:expr ) => {
        match auth_helpers::verify_jwt(Some($request.header.authorization.clone()), $state) {
            Ok(ctx) => ctx,
            Err(err) => {
                return Err(anyhow::anyhow!("Unauthorized: {}", err.message));
            }
        }
    };
}

macro_rules! handle_with_logging {
    ( $handler:expr ) => {
        match $handler.await {
            Ok(response) => Ok(response),
            Err(err) => {
                tracing::error!("Handler error: {:?}", err);
                Err(anyhow::anyhow!("Internal error: {}", err.to_string().lines().next().unwrap_or("unknown error")))
            }
        }
    };
}

#[derive(Clone)]
struct ValidatingApiServer {
    inner: ApiImpl,
}

impl ApiServer for ApiImpl {
    async fn get_products(
        &self,
        request: GetProductsRequest,
    ) -> anyhow::Result<GetProductsResponse> {
        validate!(request, GetProductsResponse);
        let auth = auth!(request, &self.state);
        handle_with_logging!(handlers::products::get::handle(&auth, &self.state, request))
    }

    async fn post_products(
        &self,
        request: PostProductsRequest,
    ) -> anyhow::Result<PostProductsResponse> {
        validate!(request, PostProductsResponse);
        let auth = auth!(request, &self.state);
        handle_with_logging!(handlers::products::post::handle(&auth, &self.state, request))
    }

    async fn get_products_by_id(
        &self,
        request: GetProductsByIdRequest,
    ) -> anyhow::Result<GetProductsByIdResponse> {
        validate!(request, GetProductsByIdResponse);
        let auth = auth!(request, &self.state);
        handle_with_logging!(handlers::products::id::get::handle(&auth, &self.state, request))
    }

    async fn put_products_by_id(
        &self,
        request: PutProductsByIdRequest,
    ) -> anyhow::Result<PutProductsByIdResponse> {
        validate!(request, PutProductsByIdResponse);
        let auth = auth!(request, &self.state);
        handle_with_logging!(handlers::products::id::put::handle(&auth, &self.state, request))
    }

    async fn delete_products_by_id(
        &self,
        request: DeleteProductsByIdRequest,
    ) -> anyhow::Result<PutProductsByIdResponse> {
        validate!(request, PutProductsByIdResponse);
        let auth = auth!(request, &self.state);
        handle_with_logging!(handlers::products::id::delete::handle(&auth, &self.state, request))
    }

    async fn post_orders(&self, request: PostOrdersRequest) -> anyhow::Result<PostOrdersResponse> {
        validate!(request, PostOrdersResponse);
        let auth = auth!(request, &self.state);
        handle_with_logging!(handlers::orders::post::handle(&auth, &self.state, request))
    }

    async fn get_orders_by_id(
        &self,
        request: GetOrdersByIdRequest,
    ) -> anyhow::Result<GetOrdersByIdResponse> {
        validate!(request, GetOrdersByIdResponse);
        let auth = auth!(request, &self.state);
        handle_with_logging!(handlers::orders::id::get::handle(&auth, &self.state, request))
    }

    async fn put_orders_by_id(
        &self,
        request: PutOrdersByIdRequest,
    ) -> anyhow::Result<PutOrdersByIdResponse> {
        validate!(request, PutOrdersByIdResponse);
        let auth = auth!(request, &self.state);
        handle_with_logging!(handlers::orders::id::put::handle(&auth, &self.state, request))
    }

    async fn post_orders_by_id_cancel(
        &self,
        request: PostOrdersByIdCancelRequest,
    ) -> anyhow::Result<PostOrdersByIdCancelResponse> {
        validate!(request, PostOrdersByIdCancelResponse);
        let auth = auth!(request, &self.state);
        handle_with_logging!(handlers::orders::id::cancel::handle(&auth, &self.state, request))
    }

    async fn post_auth_login(
        &self,
        request: PostAuthLoginRequest,
    ) -> anyhow::Result<PostAuthLoginResponse> {
        validate!(request, PostAuthLoginResponse);
        handle_with_logging!(handlers::auth::login::handler(&self.state, request))
    }

    async fn post_auth_refresh(
        &self,
        request: PostAuthRefreshRequest,
    ) -> anyhow::Result<PostAuthLoginResponse> {
        validate!(request, PostAuthLoginResponse);
        handle_with_logging!(handlers::auth::refresh::handler(&self.state, request))
    }

    async fn post_auth_register(
        &self,
        request: PostAuthRegisterRequest,
    ) -> anyhow::Result<PostAuthRegisterResponse> {
        validate!(request, PostAuthRegisterResponse);
        handle_with_logging!(handlers::auth::register::handler(&self.state, request))
    }

    async fn post_promo_codes(
        &self,
        request: PostPromoCodesRequest,
    ) -> anyhow::Result<PostPromoCodesResponse> {
        validate!(request, PostPromoCodesResponse);
        let auth = auth!(request, &self.state);
        handle_with_logging!(handlers::promo_codes::post::handle(&auth, &self.state, request))
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let api_impl = ApiImpl::new().expect("Failed to create DB pool");

    let app = api::router(api_impl)
        .layer(axum::middleware::from_fn(middleware::logging))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid));

    let port = std::env::var("PORT")
        .unwrap_or("8000".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("Service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app).await.expect("Server error");
}
