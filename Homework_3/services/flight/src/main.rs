pub mod flight {
    pub mod v1 {
        tonic::include_proto!("flight.v1");
    }
}

use std::net::SocketAddr;
use tonic::{Request, Status, transport::Server};

mod db;
mod grpc;
mod models;
mod state;

const SERVICE_API_KEY_HEADER: &str = "x-service-api-key";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let grpc_port = std::env::var("GRPC_PORT").unwrap_or_else(|_| "50051".to_string());
    let state = state::AppState::from_env()?;
    let expected_api_key = state.service_api_key.clone();
    let grpc_service = grpc::service::FlightGrpcService::new(state);

    let addr: SocketAddr = format!("0.0.0.0:{grpc_port}").parse()?;

    tracing::info!("flight-service listening on {addr}");

    Server::builder()
        .add_service(
            flight::v1::flight_service_server::FlightServiceServer::with_interceptor(
                grpc_service,
                move |request: Request<()>| match request.metadata().get(SERVICE_API_KEY_HEADER) {
                    Some(value) if value.to_str().ok() == Some(expected_api_key.as_str()) => {
                        Ok(request)
                    }
                    _ => Err(Status::unauthenticated("invalid or missing credentials")),
                },
            ),
        )
        .serve(addr)
        .await?;

    Ok(())
}
