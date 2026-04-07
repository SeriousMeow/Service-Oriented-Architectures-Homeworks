use tonic::Code;

use crate::api;
use crate::handlers::common;
use crate::state::AppState;

pub async fn handle(
    state: &AppState,
    request: api::GetFlightsByIdRequest,
) -> anyhow::Result<api::GetFlightsByIdResponse> {
    let response = state.flight_client.get_flight(request.path.id).await;
    let response = match response {
        Ok(value) => value,
        Err(status) if common::is_circuit_open(&status) => {
            return Ok(api::GetFlightsByIdResponse::ServiceUnavailable(common::error(
                "FLIGHT_SERVICE_UNAVAILABLE",
                common::grpc_message(&status),
            )));
        }
        Err(status) if status.code() == Code::NotFound => {
            return Ok(api::GetFlightsByIdResponse::NotFound(common::error(
                "FLIGHT_NOT_FOUND",
                common::grpc_message(&status),
            )));
        }
        Err(status) if status.code() == Code::InvalidArgument => {
            return Ok(api::GetFlightsByIdResponse::NotFound(common::error(
                "INVALID_ARGUMENT",
                common::grpc_message(&status),
            )));
        }
        Err(status) => {
            return Ok(api::GetFlightsByIdResponse::ServiceUnavailable(common::error(
                "FLIGHT_SERVICE_UNAVAILABLE",
                common::grpc_message(&status),
            )));
        }
    };

    let flight = response
        .flight
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("flight payload is missing"))?;

    Ok(api::GetFlightsByIdResponse::Ok(common::map_flight(flight)?))
}
