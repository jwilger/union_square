use axum::Json;

pub fn parse_boundary_payload(payload: Json<String>) -> String {
    payload.0
}
