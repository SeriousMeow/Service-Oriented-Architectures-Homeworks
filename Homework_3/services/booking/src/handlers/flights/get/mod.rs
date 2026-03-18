use chrono::NaiveTime;
use tonic::Code;

use crate::api;
use crate::handlers::common;
use crate::state::AppState;

pub async fn handle(
    state: &AppState,
    request: api::GetFlightsRequest,
) -> anyhow::Result<api::GetFlightsResponse> {
    let date = request.query.date.map(|value| {
        let dt = value.and_time(NaiveTime::MIN).and_utc();
        prost_types::Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        }
    });

    let response = state
        .flight_client
        .search_flights(
            request.query.origin.to_uppercase(),
            request.query.destination.to_uppercase(),
            date,
        )
        .await;

    let response = match response {
        Ok(value) => value,
        Err(status) if common::is_circuit_open(&status) => {
            return Ok(api::GetFlightsResponse::ServiceUnavailable(common::error(
                "FLIGHT_SERVICE_UNAVAILABLE",
                common::grpc_message(&status),
            )));
        }
        Err(status) if status.code() == Code::InvalidArgument => {
            return Ok(api::GetFlightsResponse::BadRequest(common::error(
                "INVALID_ARGUMENT",
                common::grpc_message(&status),
            )));
        }
        Err(status) => {
            return Ok(api::GetFlightsResponse::ServiceUnavailable(common::error(
                "FLIGHT_SERVICE_UNAVAILABLE",
                common::grpc_message(&status),
            )));
        }
    };

    let mut items = Vec::with_capacity(response.flights.len());
    for flight in &response.flights {
        items.push(common::map_flight(flight)?);
    }

    Ok(api::GetFlightsResponse::Ok(api::FlightListResponse {
        items,
    }))
}
