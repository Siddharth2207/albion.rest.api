use super::{
    build_order_summary, build_pagination, OrdersListDataSource, RaindexOrdersListDataSource,
    DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE,
};
use crate::auth::AuthenticatedKey;
use crate::error::{ApiError, ApiErrorResponse};
use crate::fairings::{GlobalRateLimit, TracingSpan};
use crate::types::common::ValidatedAddress;
use crate::types::orders::{OrderSide, OrdersByTokenParams, OrdersListResponse};
use alloy::primitives::Address;
use futures::future::join_all;
use rain_orderbook_common::raindex_client::orders::GetOrdersFilters;
use rain_orderbook_common::raindex_client::orders::GetOrdersTokenFilter;
use rocket::serde::json::Json;
use rocket::State;
use std::time::Instant;
use tracing::Instrument;

pub(crate) async fn process_get_orders_by_token(
    ds: &dyn OrdersListDataSource,
    address: Address,
    side: Option<OrderSide>,
    page: Option<u16>,
    page_size: Option<u16>,
) -> Result<OrdersListResponse, ApiError> {
    let total_start = Instant::now();
    let token_filter = match side {
        Some(OrderSide::Input) => GetOrdersTokenFilter {
            inputs: Some(vec![address]),
            outputs: None,
        },
        Some(OrderSide::Output) => GetOrdersTokenFilter {
            inputs: None,
            outputs: Some(vec![address]),
        },
        None => GetOrdersTokenFilter {
            inputs: Some(vec![address]),
            outputs: Some(vec![address]),
        },
    };

    let filters = GetOrdersFilters {
        active: Some(true),
        tokens: Some(token_filter),
        ..Default::default()
    };

    let page_num = page.unwrap_or(1);
    let effective_page_size = page_size
        .unwrap_or(DEFAULT_PAGE_SIZE as u16)
        .min(MAX_PAGE_SIZE);

    let orders_stage_start = Instant::now();
    let (orders, total_count) = ds
        .get_orders_list(filters, Some(page_num), Some(effective_page_size))
        .await?;
    let orders_stage_duration_ms = orders_stage_start.elapsed().as_millis();

    let quotes_stage_start = Instant::now();
    let quote_futures: Vec<_> = orders.iter().map(|o| ds.get_order_quotes(o)).collect();
    let quote_results = join_all(quote_futures).await;
    let quotes_stage_duration_ms = quotes_stage_start.elapsed().as_millis();

    let mut summaries = Vec::with_capacity(orders.len());
    let mut quote_success_count: usize = 0;
    let mut quote_empty_count: usize = 0;
    let mut quote_error_count: usize = 0;
    for (order, quotes_result) in orders.iter().zip(quote_results) {
        let io_ratio = match quotes_result {
            Ok(quotes) => quotes
                .first()
                .and_then(|q| q.data.as_ref())
                .map(|d| {
                    quote_success_count += 1;
                    d.formatted_ratio.clone()
                })
                .unwrap_or_else(|| {
                    quote_empty_count += 1;
                    "-".into()
                }),
            Err(err) => {
                quote_error_count += 1;
                tracing::warn!(
                    order_hash = ?order.order_hash(),
                    error = ?err,
                    "quote fetch failed; using fallback io_ratio"
                );
                "-".into()
            }
        };
        summaries.push(build_order_summary(order, &io_ratio)?);
    }

    let pagination = build_pagination(total_count, page_num.into(), effective_page_size.into());
    tracing::info!(
        page = page_num,
        page_size = effective_page_size,
        returned_orders = summaries.len(),
        total_orders = total_count,
        quote_success_count,
        quote_empty_count,
        quote_error_count,
        orders_stage_duration_ms,
        quotes_stage_duration_ms,
        total_duration_ms = total_start.elapsed().as_millis(),
        "orders by token request processed"
    );
    Ok(OrdersListResponse {
        orders: summaries,
        pagination,
    })
}

#[utoipa::path(
    get,
    path = "/v1/orders/token/{address}",
    tag = "Orders",
    security(("basicAuth" = [])),
    params(
        ("address" = String, Path, description = "Token address"),
        OrdersByTokenParams,
    ),
    responses(
        (status = 200, description = "Paginated list of orders for token", body = OrdersListResponse),
        (status = 400, description = "Bad request", body = ApiErrorResponse),
        (status = 422, description = "Unprocessable entity", body = ApiErrorResponse),
        (status = 401, description = "Unauthorized", body = ApiErrorResponse),
        (status = 429, description = "Rate limited", body = ApiErrorResponse),
        (status = 500, description = "Internal server error", body = ApiErrorResponse),
    )
)]
#[get("/token/<address>?<params..>")]
pub async fn get_orders_by_token(
    _global: GlobalRateLimit,
    _key: AuthenticatedKey,
    shared_raindex: &State<crate::raindex::SharedRaindexProvider>,
    span: TracingSpan,
    address: ValidatedAddress,
    params: OrdersByTokenParams,
) -> Result<Json<OrdersListResponse>, ApiError> {
    async move {
        tracing::info!(address = ?address, params = ?params, "request received");
        let handler_start = Instant::now();
        let addr = address.0;
        let side = params.side;
        let page = params.page;
        let page_size = params.page_size;
        let raindex_lock_start = Instant::now();
        let raindex = shared_raindex.read().await;
        let raindex_lock_wait_duration_ms = raindex_lock_start.elapsed().as_millis();
        if let Some(path) = raindex.db_path() {
            let db_stat_start = Instant::now();
            match std::fs::metadata(&path) {
                Ok(metadata) => {
                    tracing::info!(
                        local_db_path = %path.display(),
                        local_db_size_bytes = metadata.len(),
                        local_db_stat_duration_ms = db_stat_start.elapsed().as_millis(),
                        raindex_lock_wait_duration_ms,
                        "local raindex db size observed"
                    );
                }
                Err(error) => {
                    tracing::warn!(
                        local_db_path = %path.display(),
                        error = %error,
                        local_db_stat_duration_ms = db_stat_start.elapsed().as_millis(),
                        raindex_lock_wait_duration_ms,
                        "failed to stat local raindex db path"
                    );
                }
            }
        } else {
            tracing::info!(
                raindex_lock_wait_duration_ms,
                "raindex provider has no local db path configured"
            );
        }
        let ds = RaindexOrdersListDataSource {
            client: raindex.client(),
        };
        let response = process_get_orders_by_token(&ds, addr, side, page, page_size).await?;
        tracing::info!(
            duration_ms = handler_start.elapsed().as_millis(),
            "orders by token handler completed"
        );
        Ok(Json(response))
    }
    .instrument(span.0)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::order::test_fixtures::{
        mock_order, mock_order_with_shared_vaults, mock_quote,
    };
    use crate::routes::orders::test_fixtures::MockOrdersListDataSource;
    use crate::test_helpers::{basic_auth_header, seed_api_key, TestClientBuilder};
    use rocket::http::{Header, Status};

    #[rocket::async_test]
    async fn test_process_get_orders_by_token_success() {
        let ds = MockOrdersListDataSource {
            orders: Ok(vec![mock_order()]),
            total_count: 1,
            quotes: Ok(vec![mock_quote("1.5")]),
        };
        let addr: Address = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
            .parse()
            .unwrap();
        let result = process_get_orders_by_token(&ds, addr, None, None, None)
            .await
            .unwrap();

        assert_eq!(result.orders.len(), 1);
        assert_eq!(result.orders[0].input_token.symbol, "USDC");
        assert_eq!(result.orders[0].output_token.symbol, "WETH");
        assert_eq!(result.orders[0].io_ratio, "1.5");
        assert_eq!(result.pagination.total_orders, 1);
        assert_eq!(result.pagination.page, 1);
        assert!(!result.pagination.has_more);
    }

    #[rocket::async_test]
    async fn test_process_get_orders_by_token_empty() {
        let ds = MockOrdersListDataSource {
            orders: Ok(vec![]),
            total_count: 0,
            quotes: Ok(vec![]),
        };
        let addr: Address = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
            .parse()
            .unwrap();
        let result = process_get_orders_by_token(&ds, addr, Some(OrderSide::Input), None, None)
            .await
            .unwrap();

        assert!(result.orders.is_empty());
        assert_eq!(result.pagination.total_orders, 0);
        assert_eq!(result.pagination.total_pages, 0);
    }

    #[rocket::async_test]
    async fn test_process_get_orders_by_token_quote_failure_shows_dash() {
        let ds = MockOrdersListDataSource {
            orders: Ok(vec![mock_order()]),
            total_count: 1,
            quotes: Err(ApiError::Internal("quote error".into())),
        };
        let addr: Address = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
            .parse()
            .unwrap();
        let result = process_get_orders_by_token(&ds, addr, None, None, None)
            .await
            .unwrap();

        assert_eq!(result.orders[0].io_ratio, "-");
    }

    #[rocket::async_test]
    async fn test_process_get_orders_by_token_query_failure() {
        let ds = MockOrdersListDataSource {
            orders: Err(ApiError::Internal("failed".into())),
            total_count: 0,
            quotes: Ok(vec![]),
        };
        let addr: Address = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
            .parse()
            .unwrap();
        let result = process_get_orders_by_token(&ds, addr, None, None, None).await;
        assert!(matches!(result, Err(ApiError::Internal(_))));
    }

    #[rocket::async_test]
    async fn test_process_get_orders_by_token_shared_vaults() {
        let ds = MockOrdersListDataSource {
            orders: Ok(vec![mock_order_with_shared_vaults()]),
            total_count: 1,
            quotes: Ok(vec![mock_quote("200.0")]),
        };
        let addr: Address = "0xff05e1bd696900dc6a52ca35ca61bb1024eda8e2"
            .parse()
            .unwrap();
        let result = process_get_orders_by_token(&ds, addr, None, None, None)
            .await
            .unwrap();

        assert_eq!(result.orders.len(), 1);
        assert_eq!(result.orders[0].input_token.symbol, "wtMSTR");
        assert_eq!(result.orders[0].output_token.symbol, "wtMSTR");
    }

    #[rocket::async_test]
    async fn test_pagination_math() {
        use super::super::build_pagination;

        let p = build_pagination(250, 1, 100);
        assert_eq!(p.total_orders, 250);
        assert_eq!(p.total_pages, 3);
        assert!(p.has_more);

        let p = build_pagination(250, 3, 100);
        assert_eq!(p.total_pages, 3);
        assert!(!p.has_more);

        let p = build_pagination(0, 1, 100);
        assert_eq!(p.total_pages, 0);
        assert!(!p.has_more);

        let p = build_pagination(100, 1, 100);
        assert_eq!(p.total_pages, 1);
        assert!(!p.has_more);

        let p = build_pagination(101, 1, 100);
        assert_eq!(p.total_pages, 2);
        assert!(p.has_more);
    }

    #[rocket::async_test]
    async fn test_pagination_math_single_item() {
        use super::super::build_pagination;

        let p = build_pagination(1, 1, 50);
        assert_eq!(p.total_orders, 1);
        assert_eq!(p.total_pages, 1);
        assert!(!p.has_more);
        assert_eq!(p.page, 1);
        assert_eq!(p.page_size, 50);
    }

    #[rocket::async_test]
    async fn test_pagination_math_exactly_fills_pages() {
        use super::super::build_pagination;

        let p = build_pagination(200, 1, 50);
        assert_eq!(p.total_pages, 4);
        assert!(p.has_more);

        let p = build_pagination(200, 4, 50);
        assert_eq!(p.total_pages, 4);
        assert!(!p.has_more);
    }

    #[rocket::async_test]
    async fn test_pagination_math_page_size_zero_returns_zero_pages() {
        use super::super::build_pagination;

        // page_size=0 is a degenerate case — should not panic or divide by zero
        let p = build_pagination(100, 1, 0);
        assert_eq!(p.total_pages, 0, "page_size=0 should yield 0 total_pages");
        assert!(!p.has_more);
    }

    #[rocket::async_test]
    async fn test_pagination_math_large_total() {
        use super::super::build_pagination;

        let p = build_pagination(u32::MAX, 1, 50);
        assert!(p.total_pages > 0, "large total_count should not overflow");
        assert!(p.has_more);
    }

    #[rocket::async_test]
    async fn test_process_get_orders_by_token_page_zero_treated_as_page_one() {
        let ds = MockOrdersListDataSource {
            orders: Ok(vec![mock_order()]),
            total_count: 1,
            quotes: Ok(vec![mock_quote("1.5")]),
        };
        let addr: Address = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
            .parse()
            .unwrap();
        let result = process_get_orders_by_token(&ds, addr, None, Some(0), None)
            .await
            .unwrap();

        assert!(
            result.pagination.page <= 1,
            "page=0 should be treated as page 1, got {}",
            result.pagination.page
        );
    }

    #[rocket::async_test]
    async fn test_process_get_orders_by_token_all_sides() {
        for side in [None, Some(OrderSide::Input), Some(OrderSide::Output)] {
            let ds = MockOrdersListDataSource {
                orders: Ok(vec![mock_order()]),
                total_count: 1,
                quotes: Ok(vec![mock_quote("1.5")]),
            };
            let addr: Address = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
                .parse()
                .unwrap();
            let result = process_get_orders_by_token(&ds, addr, side, None, None)
                .await
                .unwrap();
            assert_eq!(
                result.orders.len(),
                1,
                "should return 1 order for side {:?}",
                side
            );
        }
    }

    #[rocket::async_test]
    async fn test_get_orders_by_token_401_without_auth() {
        let client = TestClientBuilder::new().build().await;
        let response = client
            .get("/v1/orders/token/0x833589fcd6edb6e08f4c7c32d4f71b54bda02913")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    async fn test_get_orders_by_token_invalid_address_returns_404() {
        let client = TestClientBuilder::new().build().await;
        let (key_id, secret) = seed_api_key(&client).await;
        let header = basic_auth_header(&key_id, &secret);
        let response = client
            .get("/v1/orders/token/not-an-address")
            .header(Header::new("Authorization", header))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::UnprocessableEntity);
    }

    #[rocket::async_test]
    async fn test_process_get_orders_by_token_explicit_page_and_page_size() {
        let ds = MockOrdersListDataSource {
            orders: Ok(vec![mock_order()]),
            total_count: 200,
            quotes: Ok(vec![mock_quote("1.5")]),
        };
        let addr: Address = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
            .parse()
            .unwrap();
        let result = process_get_orders_by_token(&ds, addr, None, Some(3), Some(25))
            .await
            .unwrap();

        assert_eq!(result.pagination.page, 3);
        assert_eq!(result.pagination.page_size, 25);
        assert_eq!(result.pagination.total_orders, 200);
        assert_eq!(result.pagination.total_pages, 8);
        assert!(result.pagination.has_more);
    }

    #[rocket::async_test]
    async fn test_process_get_orders_by_token_page_size_capped_at_max() {
        let ds = MockOrdersListDataSource {
            orders: Ok(vec![]),
            total_count: 0,
            quotes: Ok(vec![]),
        };
        let addr: Address = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
            .parse()
            .unwrap();
        let result = process_get_orders_by_token(&ds, addr, None, Some(1), Some(999))
            .await
            .unwrap();

        assert_eq!(
            result.pagination.page_size,
            super::super::MAX_PAGE_SIZE as u32,
            "page_size should be capped at MAX_PAGE_SIZE"
        );
    }

    #[rocket::async_test]
    async fn test_process_get_orders_by_token_last_page_has_more_false() {
        let ds = MockOrdersListDataSource {
            orders: Ok(vec![mock_order()]),
            total_count: 50,
            quotes: Ok(vec![mock_quote("1.5")]),
        };
        let addr: Address = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
            .parse()
            .unwrap();
        let result = process_get_orders_by_token(&ds, addr, Some(OrderSide::Output), Some(2), Some(25))
            .await
            .unwrap();

        assert_eq!(result.pagination.page, 2);
        assert_eq!(result.pagination.total_pages, 2);
        assert!(!result.pagination.has_more);
    }
}
