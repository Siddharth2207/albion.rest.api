use crate::error::{ApiErrorDetail, ApiErrorResponse};
use crate::fairings::{request_id_for, request_span_for};
use rocket::http::Header;
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::Catcher;
use rocket::Request;

#[catch(400)]
pub fn bad_request(req: &Request<'_>) -> Json<ApiErrorResponse> {
    let span = request_span_for(req);
    span.in_scope(|| {
        tracing::warn!("bad request (invalid content type, missing headers, or malformed input)")
    });

    Json(ApiErrorResponse {
        request_id: request_id_for(req),
        error: ApiErrorDetail {
            code: "BAD_REQUEST".to_string(),
            message: "The request was invalid or malformed".to_string(),
        },
    })
}

#[catch(401)]
pub fn unauthorized(req: &Request<'_>) -> Json<ApiErrorResponse> {
    let span = request_span_for(req);
    span.in_scope(|| tracing::warn!("unauthorized (missing or invalid credentials)"));

    Json(ApiErrorResponse {
        request_id: request_id_for(req),
        error: ApiErrorDetail {
            code: "UNAUTHORIZED".to_string(),
            message: "Missing or invalid credentials".to_string(),
        },
    })
}

#[catch(403)]
pub fn forbidden(req: &Request<'_>) -> Json<ApiErrorResponse> {
    let span = request_span_for(req);
    span.in_scope(|| tracing::warn!("forbidden (insufficient permissions)"));

    Json(ApiErrorResponse {
        request_id: request_id_for(req),
        error: ApiErrorDetail {
            code: "FORBIDDEN".to_string(),
            message: "Insufficient permissions".to_string(),
        },
    })
}

#[catch(404)]
pub fn not_found(req: &Request<'_>) -> Json<ApiErrorResponse> {
    let span = request_span_for(req);
    span.in_scope(|| tracing::warn!("route not found"));

    Json(ApiErrorResponse {
        request_id: request_id_for(req),
        error: ApiErrorDetail {
            code: "NOT_FOUND".to_string(),
            message: "The requested resource was not found".to_string(),
        },
    })
}

#[catch(422)]
pub fn unprocessable_entity(req: &Request<'_>) -> Json<ApiErrorResponse> {
    let span = request_span_for(req);
    span.in_scope(|| tracing::warn!("unprocessable entity (likely malformed request body)"));

    Json(ApiErrorResponse {
        request_id: request_id_for(req),
        error: ApiErrorDetail {
            code: "UNPROCESSABLE_ENTITY".to_string(),
            message: "Request body could not be parsed".to_string(),
        },
    })
}

pub(crate) struct RateLimitedResponse(Json<ApiErrorResponse>);

impl<'r, 'o: 'r> Responder<'r, 'o> for RateLimitedResponse {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'o> {
        let mut res = self.0.respond_to(req)?;
        res.set_header(Header::new("Retry-After", "60"));
        Ok(res)
    }
}

#[catch(429)]
pub fn too_many_requests(req: &Request<'_>) -> RateLimitedResponse {
    let span = request_span_for(req);
    span.in_scope(|| tracing::warn!("rate limit exceeded"));

    RateLimitedResponse(Json(ApiErrorResponse {
        request_id: request_id_for(req),
        error: ApiErrorDetail {
            code: "RATE_LIMITED".to_string(),
            message: "Too many requests, please try again later".to_string(),
        },
    }))
}

#[catch(500)]
pub fn internal_server_error(req: &Request<'_>) -> Json<ApiErrorResponse> {
    let span = request_span_for(req);
    span.in_scope(|| tracing::error!("unhandled internal server error"));

    Json(ApiErrorResponse {
        request_id: request_id_for(req),
        error: ApiErrorDetail {
            code: "INTERNAL_ERROR".to_string(),
            message: "Internal server error".to_string(),
        },
    })
}

pub fn catchers() -> Vec<Catcher> {
    rocket::catchers![
        bad_request,
        unauthorized,
        forbidden,
        not_found,
        too_many_requests,
        unprocessable_entity,
        internal_server_error
    ]
}

#[cfg(test)]
mod tests {
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    /// Build a minimal Rocket with only catchers registered, no routes.
    /// Any request will hit a catcher since there are no matching routes.
    fn catcher_client() -> Client {
        let rocket = rocket::build().register("/", super::catchers());
        Client::tracked(rocket).expect("valid rocket for catcher tests")
    }

    fn parse_json(raw: &str) -> serde_json::Value {
        serde_json::from_str(raw).expect("response must be valid JSON")
    }

    fn assert_catcher_shape(body: &serde_json::Value, expected_code: &str) {
        assert!(
            body["request_id"].is_string(),
            "catcher must include request_id"
        );
        assert!(
            body["error"].is_object(),
            "catcher must include error object"
        );
        assert_eq!(
            body["error"]["code"].as_str().unwrap(),
            expected_code,
            "unexpected error code"
        );
        assert!(
            body["error"]["message"].is_string(),
            "error.message must be a string"
        );
    }

    #[test]
    fn test_404_catcher_shape() {
        let client = catcher_client();
        let response = client.get("/nonexistent").dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let body = parse_json(&response.into_string().unwrap());
        assert_catcher_shape(&body, "NOT_FOUND");
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    #[test]
    fn test_404_catcher_content_type_is_json() {
        let client = catcher_client();
        let response = client.get("/nonexistent").dispatch();
        let ct = response.content_type();
        assert!(ct.is_some());
        assert!(ct.unwrap().is_json());
    }

    #[test]
    fn test_catchers_vec_contains_expected_count() {
        let catchers = super::catchers();
        // We register 7 catchers: 400, 401, 403, 404, 422, 429, 500
        assert_eq!(
            catchers.len(),
            7,
            "expected 7 catchers (400, 401, 403, 404, 422, 429, 500)"
        );
    }

    #[test]
    fn test_404_request_id_is_non_empty() {
        let client = catcher_client();
        let response = client.get("/does-not-exist").dispatch();
        let body = parse_json(&response.into_string().unwrap());
        let request_id = body["request_id"].as_str().unwrap();
        assert!(!request_id.is_empty(), "request_id must not be empty");
    }

    #[test]
    fn test_404_unique_request_ids() {
        let client = catcher_client();
        let r1 = client.get("/a").dispatch();
        let body1 = parse_json(&r1.into_string().unwrap());
        let r2 = client.get("/b").dispatch();
        let body2 = parse_json(&r2.into_string().unwrap());

        let id1 = body1["request_id"].as_str().unwrap();
        let id2 = body2["request_id"].as_str().unwrap();
        assert_ne!(id1, id2, "different requests should get different request_ids");
    }
}
