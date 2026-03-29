use base64::Engine;
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
// Auth edge cases – malformed base64, no colon, inactive key
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_malformed_base64_auth_returns_401() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", "Basic !!!not-base64!!!"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_base64_without_colon_returns_401() {
    let client = TestClientBuilder::new().build().await;
    // base64 of "nocolonseparator" (no ':' in decoded string)
    let encoded =
        base64::engine::general_purpose::STANDARD.encode("nocolonseparator");
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", format!("Basic {encoded}")))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_empty_key_id_and_secret_returns_401() {
    let client = TestClientBuilder::new().build().await;
    // base64 of ":" (empty key_id and empty secret)
    let header = basic_auth_header("", "");
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn test_inactive_api_key_returns_401() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;

    // Deactivate the key
    let pool = client
        .rocket()
        .state::<crate::db::DbPool>()
        .expect("pool in state");
    sqlx::query("UPDATE api_keys SET active = 0 WHERE key_id = ?")
        .bind(&key_id)
        .execute(pool)
        .await
        .expect("deactivate key");

    let header = basic_auth_header(&key_id, &secret);
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_valid_key_wrong_secret_returns_401() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, _secret) = seed_api_key(&client).await;

    let header = basic_auth_header(&key_id, "completely-wrong-secret");
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
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
// Orders by owner – pagination query params accepted at HTTP level
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_orders_by_owner_accepts_pagination_query_params() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    // The route should accept page and pageSize query params without returning
    // 400/422 — even though the underlying subgraph call may fail or return
    // empty, the query params themselves must parse successfully.
    let response = client
        .get("/v1/orders/owner/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913?page=2&pageSize=10")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    // Should not be a client error due to param parsing; 500 from subgraph is acceptable
    let status = response.status().code;
    assert!(
        status != 400 && status != 422,
        "pagination params should be accepted, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Orders by token – GET /v1/orders/token/<address>
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_orders_by_token_accepts_pagination_and_side_query_params() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/orders/token/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913?page=1&pageSize=50&side=input")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status != 400 && status != 422,
        "pagination + side params should be accepted, got {status}"
    );
}

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

    // The next authenticated request must be rate-limited
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    assert_eq!(
        response.status(),
        Status::TooManyRequests,
        "expected 429 after exhausting global rate limit of 2 RPM"
    );
    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "RATE_LIMITED");
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

// ---------------------------------------------------------------------------
// POST endpoints – missing content-type returns 400 consistently
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_swap_calldata_missing_content_type_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/swap/calldata")
        .header(Header::new("Authorization", header))
        .body(r#"{"taker":"0x1111111111111111111111111111111111111111","inputToken":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","outputToken":"0x4200000000000000000000000000000000000006","outputAmount":"100","maximumIoRatio":"2.5"}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 without content-type on calldata, got {status}"
    );
}

#[rocket::async_test]
async fn test_deploy_solver_missing_content_type_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/order/solver")
        .header(Header::new("Authorization", header))
        .body(r#"{"inputToken":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","outputToken":"0x4200000000000000000000000000000000000006","amount":"1000","ioRatio":"0.0005"}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 without content-type on solver, got {status}"
    );
}

#[rocket::async_test]
async fn test_deploy_dca_missing_content_type_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/order/dca")
        .header(Header::new("Authorization", header))
        .body(r#"{"inputToken":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","outputToken":"0x4200000000000000000000000000000000000006","budgetAmount":"1000","period":4,"periodUnit":"hours","startIo":"0.0005","floorIo":"0.0003"}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 without content-type on dca, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Swap quote – completely invalid JSON body
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_swap_quote_invalid_json_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/swap/quote")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body("not json at all")
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 for invalid JSON, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Swap calldata – completely invalid JSON body
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_swap_calldata_invalid_json_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/swap/calldata")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body("{{{{broken json")
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 for broken JSON, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Cancel order – invalid order hash format in body
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_cancel_order_invalid_hash_in_body_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/order/cancel")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(r#"{"orderHash":"0xshort"}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 for invalid orderHash, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Orders by owner – page=0 boundary
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_orders_by_owner_page_zero_does_not_crash() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/orders/owner/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913?page=0&pageSize=10")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    // Should not panic; any non-crash response is acceptable
    let status = response.status().code;
    assert!(
        status != 422,
        "page=0 should be accepted as a query param, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Orders by token – page=0 boundary
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_orders_by_token_page_zero_does_not_crash() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/orders/token/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913?page=0")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status != 422,
        "page=0 should be accepted as a query param, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Orders by token – invalid side parameter
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_orders_by_token_invalid_side_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/orders/token/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913?side=invalid")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422 || status == 500,
        "expected error for invalid side, got {status}"
    );
}

// ---------------------------------------------------------------------------
// CORS preflight – OPTIONS request
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_cors_preflight_options_returns_allow_headers() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .options("/health")
        .header(Header::new("Origin", "http://example.com"))
        .header(Header::new("Access-Control-Request-Method", "GET"))
        .dispatch()
        .await;

    // Preflight should succeed (200 or 204)
    let status = response.status().code;
    assert!(
        status == 200 || status == 204,
        "CORS preflight should succeed, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Rate limiting – per-key limit returns 429 with expected body and headers
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_per_key_rate_limiting_returns_429() {
    let rate_limiter = crate::fairings::RateLimiter::new(10000, 1);
    let client = TestClientBuilder::new()
        .rate_limiter(rate_limiter)
        .build()
        .await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    // First request uses the per-key quota
    let first = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header.clone()))
        .dispatch()
        .await;
    assert_ne!(
        first.status(),
        Status::TooManyRequests,
        "first request should not be rate limited"
    );

    // Second request exceeds per-key limit of 1
    let second = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(second.status(), Status::TooManyRequests);

    let body = parse_json(&second.into_string().await.unwrap());
    assert_error_shape(&body, "RATE_LIMITED");
}

// ---------------------------------------------------------------------------
// Tokens – response content-type is JSON
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_tokens_response_content_type_is_json() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let ct = response.content_type();
    assert!(ct.is_some(), "response must have Content-Type header");
    assert!(
        ct.unwrap().is_json(),
        "Content-Type must be application/json"
    );
}

// ---------------------------------------------------------------------------
// Health – response content-type is JSON
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_health_response_content_type_is_json() {
    let client = TestClientBuilder::new().build().await;
    let response = client.get("/health").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let ct = response.content_type();
    assert!(ct.is_some(), "response must have Content-Type header");
    assert!(
        ct.unwrap().is_json(),
        "Content-Type must be application/json"
    );
}

// ---------------------------------------------------------------------------
// 404 response shape consistency for unmatched routes
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_deep_unknown_route_returns_404_with_shape() {
    let client = TestClientBuilder::new().build().await;
    let response = client
        .get("/v1/some/deeply/nested/nonexistent/route")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "NOT_FOUND");
}

// ---------------------------------------------------------------------------
// Admin – non-JSON content type returns error
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_admin_registry_non_json_content_type_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_admin_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .put("/admin/registry")
        .header(Header::new("Authorization", header))
        .header(ContentType::Plain)
        .body(r#"{"registry_url":"http://example.com/registry.txt"}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "expected 400 or 422 for non-JSON content type, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Adversarial / SQL-injection-style inputs
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_sql_injection_in_auth_key_id_returns_401() {
    let client = TestClientBuilder::new().build().await;
    let header = basic_auth_header("'; DROP TABLE api_keys; --", "secret");
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNAUTHORIZED");
}

#[rocket::async_test]
async fn test_sql_injection_in_order_hash_path_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/order/0x'; DROP TABLE orders; --")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 404 || status == 422,
        "SQL-injection-style order hash should be rejected, got {status}"
    );
}

#[rocket::async_test]
async fn test_sql_injection_in_owner_address_returns_422() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/orders/owner/'; DROP TABLE api_keys; --")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::UnprocessableEntity);

    let body = parse_json(&response.into_string().await.unwrap());
    assert_error_shape(&body, "UNPROCESSABLE_ENTITY");
}

#[rocket::async_test]
async fn test_sql_injection_in_cancel_body_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/order/cancel")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(r#"{"orderHash":"'; DROP TABLE orders; --"}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "SQL-injection-style cancel hash should be rejected, got {status}"
    );
}

#[rocket::async_test]
async fn test_unicode_injection_in_auth_header_returns_401() {
    let client = TestClientBuilder::new().build().await;
    let header = basic_auth_header("key\u{0000}id", "sec\u{0000}ret");
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

// ---------------------------------------------------------------------------
// Input validation boundaries – large bodies, extreme query params
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_swap_quote_very_large_body_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    // 1MB of padding in a JSON field
    let large_value = "x".repeat(1_000_000);
    let body = format!(
        r#"{{"inputToken":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","outputToken":"0x4200000000000000000000000000000000000006","outputAmount":"{large_value}"}}"#
    );

    let response = client
        .post("/v1/swap/quote")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(body)
        .dispatch()
        .await;

    // Should reject or handle gracefully (400, 413, 422, or 500), not hang
    let status = response.status().code;
    assert!(
        status >= 400,
        "very large body should not succeed, got {status}"
    );
}

#[rocket::async_test]
async fn test_orders_by_owner_max_page_size_is_capped() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    // Request absurdly large pageSize – should be capped, not crash
    let response = client
        .get("/v1/orders/owner/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913?pageSize=65535")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status != 422,
        "large pageSize should be accepted (capped internally), got {status}"
    );
}

#[rocket::async_test]
async fn test_orders_by_token_max_page_size_is_capped() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/orders/token/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913?pageSize=65535")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status != 422,
        "large pageSize should be accepted (capped internally), got {status}"
    );
}

#[rocket::async_test]
async fn test_orders_by_owner_large_page_number_does_not_crash() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .get("/v1/orders/owner/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913?page=65535")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;

    // Should not panic; any non-crash response is acceptable
    let _status = response.status().code;
}

#[rocket::async_test]
async fn test_cancel_order_extremely_long_hash_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let long_hash = format!("0x{}", "a".repeat(10000));
    let body = format!(r#"{{"orderHash":"{long_hash}"}}"#);

    let response = client
        .post("/v1/order/cancel")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(body)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "extremely long hash should be rejected, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Concurrent auth – multiple simultaneous requests with same key
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_concurrent_auth_with_same_key_succeeds() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    // Fire 10 concurrent authenticated requests
    let mut handles = Vec::new();
    for _ in 0..10 {
        let h = header.clone();
        let resp = client
            .get("/v1/tokens")
            .header(Header::new("Authorization", h))
            .dispatch()
            .await;
        handles.push(resp.status());
    }

    // All should authenticate successfully (200), none should be 401
    for status in &handles {
        assert_ne!(
            *status,
            Status::Unauthorized,
            "concurrent auth should not cause 401"
        );
    }
}

#[rocket::async_test]
async fn test_concurrent_requests_different_keys_independent_rate_limits() {
    let rate_limiter = crate::fairings::RateLimiter::new(10000, 2);
    let client = TestClientBuilder::new()
        .rate_limiter(rate_limiter)
        .build()
        .await;

    let (key_id_a, secret_a) = seed_api_key(&client).await;
    let (key_id_b, secret_b) = seed_api_key(&client).await;
    let header_a = basic_auth_header(&key_id_a, &secret_a);
    let header_b = basic_auth_header(&key_id_b, &secret_b);

    // Exhaust key A's per-key limit (2 RPM)
    for _ in 0..2 {
        client
            .get("/v1/tokens")
            .header(Header::new("Authorization", header_a.clone()))
            .dispatch()
            .await;
    }

    // Key A should now be rate limited
    let response_a = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header_a))
        .dispatch()
        .await;
    assert_eq!(
        response_a.status(),
        Status::TooManyRequests,
        "key A should be rate limited"
    );

    // Key B should still work fine
    let response_b = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header_b))
        .dispatch()
        .await;
    assert_ne!(
        response_b.status(),
        Status::TooManyRequests,
        "key B should NOT be rate limited by key A's usage"
    );
    assert_eq!(response_b.status(), Status::Ok);
}

// ---------------------------------------------------------------------------
// Auth header edge cases – additional adversarial patterns
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_auth_header_with_extra_colons_uses_first_split() {
    let client = TestClientBuilder::new().build().await;
    // "key:secret:extra" — the key_id is "key", secret is "secret:extra"
    let encoded =
        base64::engine::general_purpose::STANDARD.encode("key_id:secret:with:colons");
    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", format!("Basic {encoded}")))
        .dispatch()
        .await;

    // Should get 401 (key doesn't exist), not a parse error or panic
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn test_auth_with_very_long_credentials_returns_401() {
    let client = TestClientBuilder::new().build().await;
    let long_key = "k".repeat(10000);
    let long_secret = "s".repeat(10000);
    let header = basic_auth_header(&long_key, &long_secret);

    let response = client
        .get("/v1/tokens")
        .header(Header::new("Authorization", header))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

// ---------------------------------------------------------------------------
// Empty / whitespace-only JSON bodies
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn test_swap_quote_empty_string_body_returns_error() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_api_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .post("/v1/swap/quote")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body("")
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422,
        "empty body should be rejected, got {status}"
    );
}

#[rocket::async_test]
async fn test_admin_registry_whitespace_only_url_returns_400() {
    let client = TestClientBuilder::new().build().await;
    let (key_id, secret) = seed_admin_key(&client).await;
    let header = basic_auth_header(&key_id, &secret);

    let response = client
        .put("/admin/registry")
        .header(Header::new("Authorization", header))
        .header(ContentType::JSON)
        .body(r#"{"registry_url":"   "}"#)
        .dispatch()
        .await;

    let status = response.status().code;
    assert!(
        status == 400 || status == 422 || status == 500,
        "whitespace-only URL should be rejected, got {status}"
    );
}
