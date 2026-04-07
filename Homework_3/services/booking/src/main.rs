pub mod flight {
    pub mod v1 {
        tonic::include_proto!("flight.v1");
    }
}

mod api;
mod db;
mod grpc;
mod handlers;
mod models;
mod state;

use std::net::SocketAddr;

use crate::state::AppState;
use api::ApiServer;
use validator::Validate;

#[derive(Clone)]
struct ApiImpl {
    state: AppState,
}

impl ApiImpl {
    fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            state: AppState::from_env()?,
        })
    }
}

macro_rules! handle_with_logging {
    ( $handler:expr ) => {
        match $handler.await {
            Ok(response) => Ok(response),
            Err(err) => {
                tracing::error!("Handler error: {:?}", err);
                Err(anyhow::anyhow!(
                    "Internal error: {}",
                    err.to_string().lines().next().unwrap_or("unknown error")
                ))
            }
        }
    };
}

fn validation_error(message: impl Into<String>) -> api::ErrorResponse {
    api::ErrorResponse {
        code: "VALIDATION_ERROR".to_string(),
        message: message.into(),
    }
}

impl ApiServer for ApiImpl {
    async fn get_bookings(
        &self,
        request: api::GetBookingsRequest,
    ) -> anyhow::Result<api::GetBookingsResponse> {
        if let Err(error) = request.validate() {
            return Ok(api::GetBookingsResponse::BadRequest(validation_error(
                format!("invalid query: {error}"),
            )));
        }
        handle_with_logging!(handlers::bookings::get::handle(&self.state, request))
    }

    async fn post_bookings(
        &self,
        request: api::PostBookingsRequest,
    ) -> anyhow::Result<api::PostBookingsResponse> {
        if let Err(error) = request.validate() {
            return Ok(api::PostBookingsResponse::BadRequest(validation_error(
                format!("invalid request: {error}"),
            )));
        }
        handle_with_logging!(handlers::bookings::post::handle(&self.state, request))
    }

    async fn get_bookings_by_id(
        &self,
        request: api::GetBookingsByIdRequest,
    ) -> anyhow::Result<api::GetBookingsByIdResponse> {
        if let Err(error) = request.validate() {
            return Ok(api::GetBookingsByIdResponse::NotFound(validation_error(
                format!("invalid path: {error}"),
            )));
        }
        handle_with_logging!(handlers::bookings::id::get::handle(&self.state, request))
    }

    async fn post_bookings_by_id_cancel(
        &self,
        request: api::PostBookingsByIdCancelRequest,
    ) -> anyhow::Result<api::PostBookingsByIdCancelResponse> {
        if let Err(error) = request.validate() {
            return Ok(api::PostBookingsByIdCancelResponse::NotFound(
                validation_error(format!("invalid path: {error}")),
            ));
        }
        handle_with_logging!(handlers::bookings::id::cancel::handle(&self.state, request))
    }

    async fn get_flights(
        &self,
        request: api::GetFlightsRequest,
    ) -> anyhow::Result<api::GetFlightsResponse> {
        if let Err(error) = request.validate() {
            return Ok(api::GetFlightsResponse::BadRequest(validation_error(
                format!("invalid query: {error}"),
            )));
        }
        handle_with_logging!(handlers::flights::get::handle(&self.state, request))
    }

    async fn get_flights_by_id(
        &self,
        request: api::GetFlightsByIdRequest,
    ) -> anyhow::Result<api::GetFlightsByIdResponse> {
        if let Err(error) = request.validate() {
            return Ok(api::GetFlightsByIdResponse::NotFound(validation_error(
                format!("invalid path: {error}"),
            )));
        }
        handle_with_logging!(handlers::flights::id::get::handle(&self.state, request))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let api_impl = ApiImpl::from_env()?;
    let app = api::router(api_impl);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("booking-service listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
