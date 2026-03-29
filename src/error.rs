use crate::fairings::{request_id_for, request_span_for};
use rocket::http::{Header, Status};
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::{Request, Response};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorDetail {
    #[schema(example = "BAD_REQUEST")]
    pub code: String,
    #[schema(example = "Something went wrong")]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"request_id": "550e8400-e29b-41d4-a716-446655440000", "error": {"code": "BAD_REQUEST", "message": "Something went wrong"}}))]
pub struct ApiErrorResponse {
    pub request_id: String,
    pub error: ApiErrorDetail,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ApiError {
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Rate limited: {0}")]
    RateLimited(String),
}

impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        let (status, code, message) = match &self {
            ApiError::BadRequest(msg) => (Status::BadRequest, "BAD_REQUEST", msg.clone()),
            ApiError::Unauthorized(msg) => (Status::Unauthorized, "UNAUTHORIZED", msg.clone()),
            ApiError::Forbidden(msg) => (Status::Forbidden, "FORBIDDEN", msg.clone()),
            ApiError::NotFound(msg) => (Status::NotFound, "NOT_FOUND", msg.clone()),
            ApiError::Internal(msg) => (Status::InternalServerError, "INTERNAL_ERROR", msg.clone()),
            ApiError::RateLimited(msg) => (Status::TooManyRequests, "RATE_LIMITED", msg.clone()),
        };
        let span = request_span_for(req);
        span.in_scope(|| {
            if status.code >= 500 {
                tracing::error!(
                    status = status.code,
                    code = %code,
                    error_message = %message,
                    "request failed"
                );
            } else {
                tracing::warn!(
                    status = status.code,
                    code = %code,
                    error_message = %message,
                    "request failed"
                );
            }
        });

        let request_id = request_id_for(req);
        let body = ApiErrorResponse {
            request_id,
            error: ApiErrorDetail {
                code: code.to_string(),
                message,
            },
        };
        let json_response = match Json(body).respond_to(req) {
            Ok(r) => r,
            Err(s) => {
                tracing::error!(status = %s.code, "failed to serialize error response");
                return Err(s);
            }
        };
        let mut response = Response::build_from(json_response)
            .status(status)
            .finalize();
        if matches!(self, ApiError::RateLimited(_)) {
            response.set_header(Header::new("Retry-After", "60"));
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::local::blocking::Client;

    #[get("/bad-request")]
    fn bad_request() -> Result<(), ApiError> {
        Err(ApiError::BadRequest("invalid input".into()))
    }
    #[get("/unauthorized")]
    fn unauthorized() -> Result<(), ApiError> {
        Err(ApiError::Unauthorized("no token".into()))
    }
    #[get("/not-found")]
    fn not_found() -> Result<(), ApiError> {
        Err(ApiError::NotFound("order not found".into()))
    }
    #[get("/internal")]
    fn internal() -> Result<(), ApiError> {
        Err(ApiError::Internal("something broke".into()))
    }

    #[get("/forbidden")]
    fn forbidden() -> Result<(), ApiError> {
        Err(ApiError::Forbidden("admin only".into()))
    }

    #[get("/rate-limited")]
    fn rate_limited() -> Result<(), ApiError> {
        Err(ApiError::RateLimited("too many requests".into()))
    }

    fn error_client() -> Client {
        let rocket = rocket::build().mount(
            "/",
            rocket::routes![bad_request, unauthorized, not_found, internal, forbidden, rate_limited],
        );
        Client::tracked(rocket).expect("valid rocket instance")
    }

    fn assert_error_response(
        client: &Client,
        path: &str,
        expected_status: u16,
        expected_code: &str,
        expected_message: &str,
    ) {
        let response = client.get(path).dispatch();
        assert_eq!(response.status().code, expected_status);
        let body: serde_json::Value =
            serde_json::from_str(&response.into_string().unwrap()).unwrap();
        assert!(body["request_id"].is_string());
        assert_eq!(body["error"]["code"], expected_code);
        assert_eq!(body["error"]["message"], expected_message);
    }

    #[test]
    fn test_bad_request_returns_400() {
        let client = error_client();
        assert_error_response(&client, "/bad-request", 400, "BAD_REQUEST", "invalid input");
    }

    #[test]
    fn test_unauthorized_returns_401() {
        let client = error_client();
        assert_error_response(&client, "/unauthorized", 401, "UNAUTHORIZED", "no token");
    }

    #[test]
    fn test_not_found_returns_404() {
        let client = error_client();
        assert_error_response(&client, "/not-found", 404, "NOT_FOUND", "order not found");
    }

    #[test]
    fn test_internal_returns_500() {
        let client = error_client();
        assert_error_response(
            &client,
            "/internal",
            500,
            "INTERNAL_ERROR",
            "something broke",
        );
    }

    #[test]
    fn test_forbidden_returns_403() {
        let client = error_client();
        assert_error_response(&client, "/forbidden", 403, "FORBIDDEN", "admin only");
    }

    #[test]
    fn test_non_rate_limited_errors_have_no_retry_after_header() {
        let client = error_client();
        let response = client.get("/bad-request").dispatch();
        assert!(
            response.headers().get_one("Retry-After").is_none(),
            "non-rate-limited errors must not have Retry-After header"
        );
        let response = client.get("/internal").dispatch();
        assert!(
            response.headers().get_one("Retry-After").is_none(),
            "non-rate-limited errors must not have Retry-After header"
        );
    }

    #[test]
    fn test_rate_limited_returns_429_with_retry_after_header() {
        let client = error_client();
        let response = client.get("/rate-limited").dispatch();
        assert_eq!(response.status().code, 429);
        let body: serde_json::Value =
            serde_json::from_str(&response.into_string().unwrap()).unwrap();
        assert_eq!(body["error"]["code"], "RATE_LIMITED");
        assert_eq!(body["error"]["message"], "too many requests");
    }

    #[test]
    fn test_rate_limited_has_retry_after_header() {
        let client = error_client();
        let response = client.get("/rate-limited").dispatch();
        let retry_after = response.headers().get_one("Retry-After");
        assert_eq!(
            retry_after,
            Some("60"),
            "RateLimited response must include Retry-After: 60 header"
        );
    }

    // -----------------------------------------------------------------------
    // Display trait – each variant produces human-readable messages
    // -----------------------------------------------------------------------

    #[test]
    fn test_bad_request_display() {
        let err = ApiError::BadRequest("invalid input".into());
        assert_eq!(err.to_string(), "Bad request: invalid input");
    }

    #[test]
    fn test_unauthorized_display() {
        let err = ApiError::Unauthorized("no token".into());
        assert_eq!(err.to_string(), "Unauthorized: no token");
    }

    #[test]
    fn test_forbidden_display() {
        let err = ApiError::Forbidden("admin only".into());
        assert_eq!(err.to_string(), "Forbidden: admin only");
    }

    #[test]
    fn test_not_found_display() {
        let err = ApiError::NotFound("order not found".into());
        assert_eq!(err.to_string(), "Not found: order not found");
    }

    #[test]
    fn test_internal_display() {
        let err = ApiError::Internal("something broke".into());
        assert_eq!(err.to_string(), "Internal error: something broke");
    }

    #[test]
    fn test_rate_limited_display() {
        let err = ApiError::RateLimited("too many requests".into());
        assert_eq!(err.to_string(), "Rate limited: too many requests");
    }

    // -----------------------------------------------------------------------
    // ApiErrorResponse – serialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_api_error_response_serializes() {
        let resp = ApiErrorResponse {
            request_id: "test-123".into(),
            error: ApiErrorDetail {
                code: "BAD_REQUEST".into(),
                message: "invalid".into(),
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"request_id\""));
        assert!(json.contains("\"code\""));
        assert!(json.contains("\"message\""));
    }

    #[test]
    fn test_api_error_response_deserializes() {
        let json = r#"{"request_id":"abc","error":{"code":"NOT_FOUND","message":"gone"}}"#;
        let resp: ApiErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.request_id, "abc");
        assert_eq!(resp.error.code, "NOT_FOUND");
        assert_eq!(resp.error.message, "gone");
    }
}
