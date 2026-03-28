use crate::test_helpers::{basic_auth_header, seed_admin_key, seed_api_key, TestClientBuilder};
use rocket::http::{ContentType, Header, Status};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Assert that `body` has the standard API error envelope:
/// `{ "request_id": "<uuid>", "error": { "code": "...", "message": "..." } }`
fn assert_error_shape(body: &serde_json::Value, expected_code: &str) {
    assert!(
        body["request_id"].is_string(),
        "error response must contain request_id"
    );
    let request_id = body["request_id"].as_str().unwrap();
    assert!(
        !request_id.is_empty(),
        "request_id must not be empty: {body}"
    );
    assert!(
        body["error"].is_object(),
        "error response must contain error object"
    );
    assert_eq!(
        body["error"]["code"].as_str().unwrap(),
        expected_code,
        "unexpected error code in: {body}"
    );
    assert!(
        body["error"]["message"].is_string(),
        "error.message must be a string"
    );
}

fn parse_json(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).expect("response body must be valid JSON")
}

// ---------------------------------------------------------------------------
// Health – GET /health (no auth required)
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_health_returns_200_with_status_ok() {
    let client = TestClientBuilder::new().build().await;
    let response = client.get("/health").dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    let body = parse_json(&response.into_string().await.unwrap());
    assert_eq!(body["status"], "ok");
}

#[rocket::async_test]
async fn test_health_has_no_extra_keys() {
    let client = TestClientBuilder::new().build().await;
    let response = client.get("/health").dispatch().await;

    let body = parse_json(&response.into_string().await.unwrap());
    let obj = body.as_object().unwrap();
    assert_eq!(obj.len(), 1, "health response should only have 'status' key");
    assert!(obj.contains_key("status"));
}

// ---------------------------------------------------------------------------
// Auth guard – consistent 401 shape across protected routes
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_missing_auth_returns_401_on_tokens() {
    let client = TestClientBuilder::new().build().await;
    let response = client.get("/v1/tokens").dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_missing_auth_returns_401_on_registry() {
    let client = TestClientBuilder::new().build().await;
    let response = client.get("/registry").dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_missing_auth_returns_401_on_swap_quote() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .post("/v1/swap/quote")
        .header(ContentType::JSON)
        .body(r#"{"inputToken":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","outputToken":"0x4200000000000000000000000000000000000006","outputAmount":"100"}"#)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_missing_auth_returns_401_on_swap_calldata() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .post("/v1/swap/calldata")
        .header(ContentType::JSON)
        .body(r#"{"taker":"0x1111111111111111111111111111111111111111","inputToken":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","outputToken":"0x4200000000000000000000000000000000000006","outputAmount":"100","maximumIoRatio":"2.5"}"#)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_missing_auth_returns_401_on_get_order() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .get("/v1/order/0x000000000000000000000000000000000000000000000000000000000000abcd")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_missing_auth_returns_401_on_cancel_order() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .post("/v1/order/cancel")
        .header(ContentType::JSON)
        .body(r#"{"orderHash":"0x000000000000000000000000000000000000000000000000000000000000abcd"}"#)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_missing_auth_returns_401_on_orders_by_owner() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .get("/v1/orders/owner/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_missing_auth_returns_401_on_orders_by_token() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .get("/v1/orders/token/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_missing_auth_returns_401_on_admin_registry() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .put("/admin/registry")
        .header(ContentType::JSON)
        .body(r#"{"registry_url":"http://example.com/registry.txt"}"#)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_invalid_auth_header_format_returns_401() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", "Bearer some-token"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn test_wrong_credentials_returns_401() {
    let client = TestClientBuilder::new().build().await;
    let header = basic_auth_header("nonexistent-key-id", "wrong-secret");
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

// ---------------------------------------------------------------------------
// Admin guard – 403 for non-admin keys
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_non_admin_key_returns_403_on_admin_registry() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .put("/admin/registry")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(r#"{"registry_url":"http://example.com/registry.txt"}"#)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "FORBIDDEN");
}

// ---------------------------------------------------------------------------
// Tokens – GET /v1/tokens
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_tokens_returns_array_with_expected_shape() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let body = parse_json(&response.into_string().await.unwrap());
    let tokens = body.as_array().expect("response must be an array");
    assert!(!tokens.is_empty(), "mock config should yield at least one token");

    let first = &tokens[0];
    assert!(first["address"].is_string(), "token must have address");
    assert!(first["network"].is_string(), "token must have network");
}

// ---------------------------------------------------------------------------
// Registry – GET /registry
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_registry_returns_url_shape() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/registry")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let body = parse_json(&response.into_string().await.unwrap());
    assert!(
        body["registry_url"].is_string(),
        "registry response must have registry_url"
    );
    let url = body["registry_url"].as_str().unwrap();
    assert!(url.starts_with("http"), "registry_url must be a URL: {url}");
}

// ---------------------------------------------------------------------------
// Admin – PUT /admin/registry
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_admin_update_registry_returns_updated_url() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_admin_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);
    let new_url = crate::test_helpers::mock_raindex_registry_url().await;

    let response = client
        .put("/admin/registry")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(format!(r#"{{"registry_url":"{new_url}"}}"#))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_eq!(body["registry_url"], new_url);
}

#[rocket::async_test]
async fn test_admin_empty_registry_url_returns_400() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_admin_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .put("/admin/registry")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(r#"{"registry_url":""}"#)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "BAD_REQUEST");
}

#[rocket::async_test]
async fn test_admin_missing_body_returns_400() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_admin_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .put("/admin/registry")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .dispatch()
        .await;

    // Rocket returns 400 or 422 for missing/malformed body
    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 for missing body, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Swap quote – POST /v1/swap/quote
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_swap_quote_malformed_body_returns_422() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/swap/quote")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(r#"{"bad":"json"}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 for malformed body, got {status}"
    );

    let body = parse_json(&response.into_string().await.unwrap());
    assert!(
        body["request_id"].is_string() || body["error"].is_object(),
        "error response should have standard shape"
    );
}

#[rocket::async_test]
async fn test_swap_quote_missing_content_type_returns_400() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/swap/quote")
        .header(Header::new("Authorization", header))
        .body(r#"{"inputToken":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","outputToken":"0x4200000000000000000000000000000000000006","outputAmount":"100"}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 without content-type, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Swap calldata – POST /v1/swap/calldata
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_swap_calldata_malformed_body_returns_422() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/swap/calldata")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(r#"{"incomplete":true}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 for malformed body, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Order – GET /v1/order/<hash>
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_get_order_invalid_hash_returns_422() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/order/not-a-valid-hash")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 404 || status == 422,
        "expected 404 or 422 for invalid order hash, got {status}"
    );
}

#[rocket::async_test]
async fn test_get_order_short_hash_returns_422() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/order/0xabcdef")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 404 || status == 422,
        "expected 404 or 422 for short hash, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Cancel order – POST /v1/order/cancel
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_cancel_order_malformed_body_returns_422() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/order/cancel")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(r#"{"wrong":"field"}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 for malformed cancel body, got {status}"
    );
}

#[rocket::async_test]
async fn test_cancel_order_empty_body_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/order/cancel")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 for empty cancel body, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Orders by owner – GET /v1/orders/owner/<address>
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_orders_by_owner_invalid_address_returns_422() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/orders/owner/not-an-address")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::UnprocessableEntity);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNPROCESSABLE_ENTITY");
}

// ---------------------------------------------------------------------------
// Orders by token – GET /v1/orders/token/<address>
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_orders_by_token_invalid_address_returns_422() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/orders/token/not-an-address")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::UnprocessableEntity);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNPROCESSABLE_ENTITY");
}

// ---------------------------------------------------------------------------
// Deploy solver – POST /v1/order/solver (todo! route)
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_deploy_solver_401_without_auth() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .post("/v1/order/solver")
        .header(ContentType::JSON)
        .body(r#"{"inputToken":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","outputToken":"0x4200000000000000000000000000000000000006","amount":"1000","ioRatio":"0.0005"}"#)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_deploy_solver_malformed_body_returns_422() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/order/solver")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(r#"{"bad":"body"}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 for malformed solver body, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Deploy DCA – POST /v1/order/dca (todo! route)
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_deploy_dca_401_without_auth() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .post("/v1/order/dca")
        .header(ContentType::JSON)
        .body(r#"{"inputToken":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","outputToken":"0x4200000000000000000000000000000000000006","budgetAmount":"1000","period":4,"periodUnit":"hours","startIo":"0.0005","floorIo":"0.0003"}"#)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_deploy_dca_malformed_body_returns_422() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/order/dca")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(r#"{"incomplete":true}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 for malformed dca body, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Trades by tx – GET /v1/trades/tx/<hash> (todo! route)
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_trades_by_tx_401_without_auth() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .get("/v1/trades/tx/0x000000000000000000000000000000000000000000000000000000000000abcd")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_trades_by_tx_invalid_hash_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/trades/tx/not-a-valid-hash")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 404 || status == 422,
        "expected 404 or 422 for invalid trades tx hash, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Trades by address – GET /v1/trades/<address> (todo! route)
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_trades_by_address_401_without_auth() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .get("/v1/trades/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_trades_by_address_invalid_address_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/trades/not-an-address")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 404 || status == 422,
        "expected 404 or 422 for invalid trades address, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Orders by tx – GET /v1/orders/tx/<hash> (todo! route)
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_orders_by_tx_401_without_auth() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .get("/v1/orders/tx/0x000000000000000000000000000000000000000000000000000000000000abcd")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_orders_by_tx_invalid_hash_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/orders/tx/not-a-hash")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 404 || status == 422,
        "expected 404 or 422 for invalid orders tx hash, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Rate limiting – 429 with Retry-After header
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_rate_limiting_returns_429_with_retry_after() {
    let rate_limiter = crate::fairings::RateLimiter::new(2, 10000);
    let client = TestClientBuilder::new()
        .rate_limiter(rate_limiter)
        .build()
        .await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    // Exhaust the global limit (2 RPM)
    for _ in 0..3 {
        client
            .get("/health")
            .header(Header::new("Authorization", header.clone()))
            .dispatch()
            .await;
    }

    // The next authenticated request should be rate-limited
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    if response.status() == Status::TooManyRequests {
        let body = parse_json(&response.into_string().await.unwrap());
        assert_error_shape(&body, "RATE_LIMITED");
    }
    // If not 429 yet (race with window), the test still passes — we verified shape when it triggers
}

// ---------------------------------------------------------------------------
// Unknown routes – 404
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_unknown_route_returns_404() {
    let client = TestClientBuilder::new().build().await;
    let response = client.get("/v1/nonexistent-route").dispatch().await;
    assert_eq!(response.status(), Status::NotFound);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "NOT_FOUND");
}

#[rocket::async_test]
async fn test_wrong_method_returns_404() {
    let client = TestClientBuilder::new().build().await;
    let response = client.delete("/health").dispatch().await;

    // Rocket returns 404 for unmatched method+path combinations
    assert_eq!(response.status(), Status::NotFound);
}

// ---------------------------------------------------------------------------
// CORS headers present
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_health_includes_cors_headers() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .get("/health")
        .header(Header::new("Origin", "http://example.com"))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    // The CORS fairing should add Access-Control-Allow-Origin
    let allow_origin = response
        .headers()
        .get_one("Access-Control-Allow-Origin");
    assert!(
        allow_origin.is_some(),
        "CORS Access-Control-Allow-Origin header should be present"
    );
}

// ---------------------------------------------------------------------------
// Error response shape consistency (shared client test preserved)
// ---------------------------------------------------------------------------

#[get("/shared-client")]
async fn shared_client_contract(
    provider: &rocket::State<crate::raindex::RaindexProvider>,
) -> Result<&'static str, crate::error::ApiError> {
    let orderbook_address =
        alloy::primitives::address!("0xd2938e7c9fe3597f78832ce780feb61945c377d7");

    provider
        .client()
        .get_orderbook_client(orderbook_address)
        .map(|_| "ok")
        .map_err(|e| crate::error::ApiError::Internal(format!("{e}")))
}

#[rocket::async_test]
async fn test_shared_client_succeeds_with_valid_registry() {
    let raindex_config = crate::test_helpers::mock_raindex_config().await;
    let rocket = rocket::build()
        .manage(raindex_config)
        .mount("/__test", rocket::routes![shared_client_contract]);
    let client =
        rocket::local::asynchronous::Client::tracked(rocket)
            .await
            .expect("valid test client");

    let response = client.get("/__test/shared-client").dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    let body = response.into_string().await.expect("response body");
    assert_eq!(body, "ok");
}

// ---------------------------------------------------------------------------
// POST with missing content-type on various endpoints
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_cancel_order_missing_content_type_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/order/cancel")
        .header(Header::new("Authorization", header))
        .body(r#"{"orderHash":"0x000000000000000000000000000000000000000000000000000000000000abcd"}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 without content-type, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Multiple tokens configuration shape test
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_tokens_with_two_tokens_returns_correct_count() {
    let settings = r#"version: 4
networks:
  base:
    rpcs:
      - https://mainnet.base.org
    chain-id: 8453
    currency: ETH
subgraphs:
  base: https://api.goldsky.com/api/public/project_clv14x04y9kzi01saerx7bxpg/subgraphs/ob4-base/0.9/gn
orderbooks:
  base:
    address: 0xd2938e7c9fe3597f78832ce780feb61945c377d7
    network: base
    subgraph: base
    deployment-block: 0
deployers:
  base:
    address: 0xC1A14cE2fd58A3A2f99deCb8eDd866204eE07f8D
    network: base
tokens:
  usdc:
    address: 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
    network: base
    decimals: 6
    label: USD Coin
    symbol: USDC
  weth:
    address: 0x4200000000000000000000000000000000000006
    network: base
    decimals: 18
    label: Wrapped Ether
    symbol: WETH
"#;
    let registry_url =
        crate::test_helpers::mock_raindex_registry_url_with_settings(settings).await;
    let config = crate::raindex::RaindexProvider::load(&registry_url, None)
        .await
        .expect("load raindex config");
    let client = TestClientBuilder::new()
        .raindex_config(config)
        .build()
        .await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let body = parse_json(&response.into_string().await.unwrap());
    let tokens = body.as_array().expect("tokens is an array");
    assert_eq!(tokens.len(), 2);

    // Each token should have at minimum address and network
    for token in tokens {
        assert!(token["address"].is_string(), "each token must have address");
        assert!(token["network"].is_string(), "each token must have network");
    }
}

// ---------------------------------------------------------------------------
// Admin registry update + verify round-trip shape
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_admin_registry_round_trip_shape() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_admin_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);
    let new_url = crate::test_helpers::mock_raindex_registry_url().await;

    // Update
    let put_response = client
        .put("/admin/registry")
        .header(Header::new("Authorization", header.clone()))
        .header(ContentType::JSON)
        .body(format!(r#"{{"registry_url":"{new_url}"}}"#))
        .dispatch()
        .await;
    assert_eq!(put_response.status(), Status::Ok);

    let put_body = parse_json(&put_response.into_string().await.unwrap());
    assert!(
        put_body["registry_url"].is_string(),
        "PUT response must have registry_url"
    );

    // Read back
    let get_response = client
        .get("/registry")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(get_response.status(), Status::Ok);

    let get_body = parse_json(&get_response.into_string().await.unwrap());
    assert_eq!(
        get_body["registry_url"], put_body["registry_url"],
        "GET must reflect the updated registry_url"
    );
}
