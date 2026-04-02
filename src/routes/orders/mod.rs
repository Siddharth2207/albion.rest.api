mod get_by_owner;
mod get_by_token;
mod get_by_tx;

use crate::error::ApiError;
use crate::types::common::TokenRef;
use crate::types::orders::{OrderSummary, OrdersPagination};
use async_trait::async_trait;
use rain_orderbook_common::raindex_client::order_quotes::RaindexOrderQuote;
use rain_orderbook_common::raindex_client::orders::{GetOrdersFilters, RaindexOrder};
use rain_orderbook_common::raindex_client::RaindexClient;
use rain_orderbook_common::raindex_client::RaindexError;
use rain_orderbook_common::rpc_client::RpcClientError;
use rocket::Route;
use std::time::Instant;

pub(crate) const DEFAULT_PAGE_SIZE: u32 = 20;
pub(crate) const MAX_PAGE_SIZE: u16 = 50;

pub(crate) struct OrderQuotesResult {
    pub quotes: Vec<RaindexOrderQuote>,
    pub duration_ms: u128,
}

struct QuoteErrorClassification {
    class: &'static str,
    upstream_http_status: Option<u16>,
    upstream_rate_limited: bool,
}

fn extract_http_status(message: &str) -> Option<u16> {
    message
        .split(|c: char| !c.is_ascii_digit())
        .find_map(|part| match part.len() {
            3 => part
                .parse::<u16>()
                .ok()
                .filter(|status| (400..600).contains(status)),
            _ => None,
        })
}

fn classify_quote_error(err: &RaindexError) -> QuoteErrorClassification {
    let message = err.to_string();
    let message_lower = message.to_ascii_lowercase();
    let upstream_http_status = extract_http_status(&message);
    let upstream_rate_limited = message.contains("429")
        || message_lower.contains("rate limit")
        || message_lower.contains("too many requests");

    let class = match err {
        RaindexError::RpcClientError(RpcClientError::RateLimited { .. }) => "upstream_rate_limited",
        _ if upstream_rate_limited => "upstream_rate_limited",
        _ => match upstream_http_status {
            Some(status) if (400..500).contains(&status) => "upstream_http_4xx",
            Some(status) if (500..600).contains(&status) => "upstream_http_5xx",
            _ => "upstream_other",
        },
    };

    QuoteErrorClassification {
        class,
        upstream_http_status,
        upstream_rate_limited,
    }
}

#[async_trait]
pub(crate) trait OrdersListDataSource: Send + Sync {
    async fn get_orders_list(
        &self,
        filters: GetOrdersFilters,
        page: Option<u16>,
        page_size: Option<u16>,
    ) -> Result<(Vec<RaindexOrder>, u32), ApiError>;

    async fn get_order_quotes(&self, order: &RaindexOrder) -> Result<OrderQuotesResult, ApiError>;
}

pub(crate) struct RaindexOrdersListDataSource<'a> {
    pub client: &'a RaindexClient,
}

#[async_trait]
impl<'a> OrdersListDataSource for RaindexOrdersListDataSource<'a> {
    async fn get_orders_list(
        &self,
        filters: GetOrdersFilters,
        page: Option<u16>,
        page_size: Option<u16>,
    ) -> Result<(Vec<RaindexOrder>, u32), ApiError> {
        let start = Instant::now();
        let result = self
            .client
            .get_orders(None, Some(filters), page, page_size)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "failed to query orders");
                ApiError::Internal("failed to query orders".into())
            })?;
        tracing::info!(
            page = page.unwrap_or(1),
            page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE as u16),
            returned_orders = result.orders().len(),
            total_orders = result.total_count(),
            duration_ms = start.elapsed().as_millis(),
            "queried orders list"
        );
        Ok((result.orders().to_vec(), result.total_count()))
    }

    async fn get_order_quotes(&self, order: &RaindexOrder) -> Result<OrderQuotesResult, ApiError> {
        let start = Instant::now();
        let order_hash = order.order_hash();
        match order.get_quotes(None, None).await {
            Ok(quotes) => {
                let duration_ms = start.elapsed().as_millis();
                tracing::info!(
                    ?order_hash,
                    quote_count = quotes.len(),
                    duration_ms,
                    "queried order quotes"
                );
                Ok(OrderQuotesResult {
                    quotes,
                    duration_ms,
                })
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis();
                let classification = classify_quote_error(&e);
                tracing::error!(
                    error = %e,
                    ?order_hash,
                    duration_ms,
                    upstream_error_class = classification.class,
                    upstream_http_status = classification.upstream_http_status,
                    upstream_rate_limited = classification.upstream_rate_limited,
                    "failed to query order quotes"
                );
                Err(ApiError::Internal("failed to query order quotes".into()))
            }
        }
    }
}

pub(crate) fn build_order_summary(
    order: &RaindexOrder,
    io_ratio: &str,
) -> Result<OrderSummary, ApiError> {
    let (input, output) = super::resolve_io_vaults(order)?;

    let input_token_info = input.token();
    let output_token_info = output.token();
    let created_at: u64 = order.timestamp_added().try_into().unwrap_or(0);

    Ok(OrderSummary {
        order_hash: order.order_hash(),
        owner: order.owner(),
        input_token: TokenRef {
            address: input_token_info.address(),
            symbol: input_token_info.symbol().unwrap_or_default(),
            decimals: input_token_info.decimals(),
        },
        output_token: TokenRef {
            address: output_token_info.address(),
            symbol: output_token_info.symbol().unwrap_or_default(),
            decimals: output_token_info.decimals(),
        },
        output_vault_balance: output.formatted_balance(),
        io_ratio: io_ratio.to_string(),
        created_at,
        orderbook_id: order.orderbook(),
    })
}

pub(crate) fn build_pagination(total_count: u32, page: u32, page_size: u32) -> OrdersPagination {
    let total_orders = total_count as u64;
    let total_pages = if page_size == 0 {
        0
    } else {
        total_orders.div_ceil(page_size as u64)
    };
    OrdersPagination {
        page,
        page_size,
        total_orders,
        total_pages,
        has_more: (page as u64) < total_pages,
    }
}

pub use get_by_owner::*;
pub use get_by_token::*;
pub use get_by_tx::*;

pub fn routes() -> Vec<Route> {
    rocket::routes![
        get_by_tx::get_orders_by_tx,
        get_by_owner::get_orders_by_address,
        get_by_token::get_orders_by_token
    ]
}

#[cfg(test)]
pub(crate) mod test_fixtures {
    use super::{OrderQuotesResult, OrdersListDataSource};
    use crate::error::ApiError;
    use async_trait::async_trait;
    use rain_orderbook_common::raindex_client::order_quotes::RaindexOrderQuote;
    use rain_orderbook_common::raindex_client::orders::{GetOrdersFilters, RaindexOrder};

    pub struct MockOrdersListDataSource {
        pub orders: Result<Vec<RaindexOrder>, ApiError>,
        pub total_count: u32,
        pub quotes: Result<Vec<RaindexOrderQuote>, ApiError>,
    }

    #[async_trait]
    impl OrdersListDataSource for MockOrdersListDataSource {
        async fn get_orders_list(
            &self,
            _filters: GetOrdersFilters,
            _page: Option<u16>,
            _page_size: Option<u16>,
        ) -> Result<(Vec<RaindexOrder>, u32), ApiError> {
            match &self.orders {
                Ok(orders) => Ok((orders.clone(), self.total_count)),
                Err(_) => Err(ApiError::Internal("failed to query orders".into())),
            }
        }

        async fn get_order_quotes(
            &self,
            _order: &RaindexOrder,
        ) -> Result<OrderQuotesResult, ApiError> {
            match &self.quotes {
                Ok(quotes) => Ok(OrderQuotesResult {
                    quotes: quotes.clone(),
                    duration_ms: 0,
                }),
                Err(_) => Err(ApiError::Internal("failed to query order quotes".into())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::extract_http_status;

    #[test]
    fn test_extract_http_status_finds_429() {
        assert_eq!(
            extract_http_status("rpc provider returned status 429 too many requests"),
            Some(429)
        );
    }

    #[test]
    fn test_extract_http_status_ignores_non_http_codes() {
        assert_eq!(extract_http_status("rpc error -32090 rate limited"), None);
    }
}
