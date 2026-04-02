use crate::auth::AuthenticatedKey;
use crate::error::{ApiError, ApiErrorResponse};
use crate::fairings::{GlobalRateLimit, TracingSpan};
use crate::types::common::{TokenRef, ValidatedAddress, ValidatedFixedBytes};
use crate::types::trades::{
    TradeByAddress, TradeByTxEntry, TradeRequest, TradeResult, TradesByAddressResponse,
    TradesByTxResponse, TradesPagination, TradesPaginationParams, TradesTotals,
};
use alloy::primitives::{Address, FixedBytes, B256};
use async_trait::async_trait;
use futures::future::join_all;
use rain_math_float::Float;
use rain_orderbook_common::raindex_client::orders::{GetOrdersFilters, RaindexOrder};
use rain_orderbook_common::raindex_client::trades::RaindexTrade;
use rain_orderbook_common::raindex_client::{RaindexClient, RaindexError};
use rocket::serde::json::Json;
use rocket::{Route, State};
use std::cmp::Reverse;
use std::ops::{Add, Div, Sub};
use std::str::FromStr;
use tracing::Instrument;

const ORDERS_SCAN_PAGE_SIZE: u16 = 50;
const FAST_INDEX_CHECK_ATTEMPTS: usize = 1;
const FAST_INDEX_CHECK_INTERVAL_MS: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxIndexState {
    Indexed,
    NotYetIndexed,
}

#[async_trait]
trait TradesDataSource: Send + Sync {
    async fn get_orders(
        &self,
        filters: GetOrdersFilters,
        page: Option<u16>,
        page_size: Option<u16>,
    ) -> Result<(Vec<RaindexOrder>, u32), ApiError>;

    async fn get_order_trades(
        &self,
        order: &RaindexOrder,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> Result<Vec<RaindexTrade>, ApiError>;

    async fn check_tx_index_state(&self, tx_hash: B256) -> Result<TxIndexState, ApiError>;
}

struct RaindexTradesDataSource<'a> {
    client: &'a RaindexClient,
}

#[async_trait]
impl TradesDataSource for RaindexTradesDataSource<'_> {
    async fn get_orders(
        &self,
        filters: GetOrdersFilters,
        page: Option<u16>,
        page_size: Option<u16>,
    ) -> Result<(Vec<RaindexOrder>, u32), ApiError> {
        let result = self
            .client
            .get_orders(None, Some(filters), page, page_size)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "failed to query orders");
                ApiError::Internal("failed to query orders".into())
            })?;
        Ok((result.orders().to_vec(), result.total_count()))
    }

    async fn get_order_trades(
        &self,
        order: &RaindexOrder,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> Result<Vec<RaindexTrade>, ApiError> {
        order
            .get_trades_list(start_time, end_time, None)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, order_hash = ?order.order_hash(), "failed to query order trades");
                ApiError::Internal("failed to query order trades".into())
            })
    }

    async fn check_tx_index_state(&self, tx_hash: B256) -> Result<TxIndexState, ApiError> {
        let orderbooks = self.client.get_all_orderbooks().map_err(|e| {
            tracing::error!(error = %e, "failed to get orderbooks");
            ApiError::Internal("failed to get orderbooks".into())
        })?;

        let mut saw_timeout = false;

        for orderbook in orderbooks.values() {
            match self
                .client
                .get_transaction(
                    orderbook.network.chain_id,
                    orderbook.address,
                    tx_hash,
                    Some(FAST_INDEX_CHECK_ATTEMPTS),
                    Some(FAST_INDEX_CHECK_INTERVAL_MS),
                )
                .await
            {
                Ok(_) => return Ok(TxIndexState::Indexed),
                Err(RaindexError::TransactionIndexingTimeout { .. }) => {
                    saw_timeout = true;
                }
                Err(err) => {
                    tracing::error!(
                        error = %err,
                        tx_hash = %tx_hash,
                        chain_id = orderbook.network.chain_id,
                        orderbook = %orderbook.address,
                        "failed to query transaction status"
                    );
                    return Err(ApiError::Internal("failed to query transaction".into()));
                }
            }
        }

        if saw_timeout {
            return Ok(TxIndexState::NotYetIndexed);
        }
        Ok(TxIndexState::Indexed)
    }
}

fn to_u64(value: alloy::primitives::U256, field: &'static str) -> Result<u64, ApiError> {
    value.try_into().map_err(|_| {
        tracing::error!(field, "value does not fit into u64");
        ApiError::Internal(format!("{field} overflow"))
    })
}

fn parse_trade_order_hash(order_hash: alloy::primitives::Bytes) -> Result<B256, ApiError> {
    let hash = order_hash.to_string();
    B256::from_str(&hash).map_err(|e| {
        tracing::error!(error = %e, order_hash = %hash, "invalid trade order hash");
        ApiError::Internal("invalid trade order hash".into())
    })
}

fn maybe_parse_trade_order_hash(order_hash: alloy::primitives::Bytes) -> Option<FixedBytes<32>> {
    FixedBytes::<32>::from_str(&order_hash.to_string()).ok()
}

fn format_float(value: Float, context: &'static str) -> Result<String, ApiError> {
    value.format().map_err(|e| {
        tracing::error!(error = %e, context, "float formatting failed");
        ApiError::Internal(format!("{context} calculation failed"))
    })
}

fn positive_output(output_amount: Float) -> Result<Float, ApiError> {
    Float::zero()
        .map_err(|e| {
            tracing::error!(error = %e, "float zero construction failed");
            ApiError::Internal("io ratio calculation failed".into())
        })?
        .sub(output_amount)
        .map_err(|e| {
            tracing::error!(error = %e, "failed to negate output amount");
            ApiError::Internal("io ratio calculation failed".into())
        })
}

fn compute_io_ratio(input_amount: Float, output_amount: Float) -> Result<String, ApiError> {
    let positive_output = positive_output(output_amount)?;
    let zero = Float::zero().map_err(|e| {
        tracing::error!(error = %e, "float zero construction failed");
        ApiError::Internal("io ratio calculation failed".into())
    })?;
    if positive_output.eq(zero).unwrap_or(true) {
        return Ok("0".into());
    }
    let ratio = input_amount.div(positive_output).map_err(|e| {
        tracing::error!(error = %e, "failed to compute io ratio");
        ApiError::Internal("io ratio calculation failed".into())
    })?;
    format_float(ratio, "io ratio")
}

async fn fetch_all_orders(
    ds: &dyn TradesDataSource,
    filters: GetOrdersFilters,
) -> Result<Vec<RaindexOrder>, ApiError> {
    let mut all_orders = Vec::new();
    let mut page: u16 = 1;

    loop {
        let (orders, total_count) = ds
            .get_orders(filters.clone(), Some(page), Some(ORDERS_SCAN_PAGE_SIZE))
            .await?;
        all_orders.extend(orders);

        if all_orders.len() >= total_count as usize {
            break;
        }
        if total_count == 0 {
            break;
        }
        page = page.saturating_add(1);
        if page == u16::MAX {
            break;
        }
    }

    Ok(all_orders)
}

struct TradeWithOwner {
    owner: Address,
    trade: RaindexTrade,
}

async fn load_trades_with_owners(
    ds: &dyn TradesDataSource,
    orders: &[RaindexOrder],
    start_time: Option<u64>,
    end_time: Option<u64>,
) -> Result<Vec<TradeWithOwner>, ApiError> {
    let trade_results = join_all(
        orders
            .iter()
            .map(|order| ds.get_order_trades(order, start_time, end_time)),
    )
    .await;

    let mut all_trades = Vec::new();
    for (order, trades_result) in orders.iter().zip(trade_results) {
        let owner = order.owner();
        for trade in trades_result? {
            all_trades.push(TradeWithOwner { owner, trade });
        }
    }

    Ok(all_trades)
}

async fn process_get_trades_by_tx(
    ds: &dyn TradesDataSource,
    tx_hash: B256,
) -> Result<Json<TradesByTxResponse>, ApiError> {
    let all_orders = fetch_all_orders(ds, GetOrdersFilters::default()).await?;
    let trades_with_owner = load_trades_with_owners(ds, &all_orders, None, None).await?;

    let mut matching_trades = Vec::new();
    for trade_with_owner in trades_with_owner {
        if trade_with_owner.trade.transaction().id() == tx_hash {
            matching_trades.push(trade_with_owner);
        }
    }

    if matching_trades.is_empty() {
        match ds.check_tx_index_state(tx_hash).await? {
            TxIndexState::NotYetIndexed => {
                return Err(ApiError::NotYetIndexed(format!(
                    "transaction {tx_hash:#x} not yet indexed"
                )));
            }
            TxIndexState::Indexed => {
                return Err(ApiError::NotFound(
                    "transaction has no associated trades".into(),
                ));
            }
        }
    }

    let first_tx = matching_trades[0].trade.transaction();
    let mut total_input = Float::zero().map_err(|e| {
        tracing::error!(error = %e, "float zero construction failed");
        ApiError::Internal("trade totals calculation failed".into())
    })?;
    let mut total_output = Float::zero().map_err(|e| {
        tracing::error!(error = %e, "float zero construction failed");
        ApiError::Internal("trade totals calculation failed".into())
    })?;

    let mut entries = Vec::with_capacity(matching_trades.len());
    for trade_with_owner in matching_trades {
        let trade = trade_with_owner.trade;
        let input_change = trade.input_vault_balance_change();
        let output_change = trade.output_vault_balance_change();
        let io_ratio = compute_io_ratio(input_change.amount(), output_change.amount())?;
        let order_hash = parse_trade_order_hash(trade.order_hash())?;

        total_input = total_input.add(input_change.amount()).map_err(|e| {
            tracing::error!(error = %e, "failed to sum total input");
            ApiError::Internal("trade totals calculation failed".into())
        })?;
        total_output = total_output
            .add(positive_output(output_change.amount())?)
            .map_err(|e| {
                tracing::error!(error = %e, "failed to sum total output");
                ApiError::Internal("trade totals calculation failed".into())
            })?;

        entries.push(TradeByTxEntry {
            order_hash,
            order_owner: trade_with_owner.owner,
            request: TradeRequest {
                input_token: input_change.token().address(),
                output_token: output_change.token().address(),
                maximum_input: input_change.formatted_amount(),
                maximum_io_ratio: io_ratio.clone(),
            },
            result: TradeResult {
                input_amount: input_change.formatted_amount(),
                output_amount: output_change.formatted_amount(),
                actual_io_ratio: io_ratio,
            },
        });
    }

    let zero = Float::zero().map_err(|e| {
        tracing::error!(error = %e, "float zero construction failed");
        ApiError::Internal("trade totals calculation failed".into())
    })?;
    let average_io_ratio = if total_output.eq(zero).unwrap_or(true) {
        zero
    } else {
        total_input.div(total_output).map_err(|e| {
            tracing::error!(error = %e, "failed to compute average io ratio");
            ApiError::Internal("trade totals calculation failed".into())
        })?
    };

    tracing::info!(
        tx_hash = %tx_hash,
        trade_count = entries.len(),
        "resolved trades by tx"
    );

    Ok(Json(TradesByTxResponse {
        tx_hash,
        block_number: to_u64(first_tx.block_number(), "block number")?,
        timestamp: to_u64(first_tx.timestamp(), "timestamp")?,
        sender: first_tx.from(),
        trades: entries,
        totals: TradesTotals {
            total_input_amount: format_float(total_input, "trade totals")?,
            total_output_amount: format_float(total_output, "trade totals")?,
            average_io_ratio: format_float(average_io_ratio, "trade totals")?,
        },
    }))
}

async fn process_get_trades_by_address(
    ds: &dyn TradesDataSource,
    owner: Address,
    params: TradesPaginationParams,
) -> Result<Json<TradesByAddressResponse>, ApiError> {
    let all_orders = fetch_all_orders(
        ds,
        GetOrdersFilters {
            owners: vec![owner],
            active: None,
            ..Default::default()
        },
    )
    .await?;

    let trades_with_owner =
        load_trades_with_owners(ds, &all_orders, params.start_time, params.end_time).await?;

    let mut trades = Vec::with_capacity(trades_with_owner.len());
    for trade_with_owner in trades_with_owner {
        let trade = trade_with_owner.trade;
        let input_change = trade.input_vault_balance_change();
        let output_change = trade.output_vault_balance_change();
        let input_token = input_change.token();
        let output_token = output_change.token();
        trades.push(TradeByAddress {
            tx_hash: trade.transaction().id(),
            input_amount: input_change.formatted_amount(),
            output_amount: output_change.formatted_amount(),
            input_token: TokenRef {
                address: input_token.address(),
                symbol: input_token.symbol().unwrap_or_default(),
                decimals: input_token.decimals(),
            },
            output_token: TokenRef {
                address: output_token.address(),
                symbol: output_token.symbol().unwrap_or_default(),
                decimals: output_token.decimals(),
            },
            order_hash: maybe_parse_trade_order_hash(trade.order_hash()),
            timestamp: to_u64(trade.timestamp(), "timestamp")?,
            block_number: to_u64(trade.transaction().block_number(), "block number")?,
        });
    }

    trades.sort_by_key(|t| (Reverse(t.timestamp), Reverse(t.block_number)));

    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let total_trades = trades.len() as u64;
    let total_pages = if page_size == 0 {
        0
    } else {
        total_trades.div_ceil(u64::from(page_size))
    };

    let offset = (u64::from(page.saturating_sub(1)) * u64::from(page_size)) as usize;
    let paginated = if offset >= trades.len() {
        Vec::new()
    } else {
        let end = std::cmp::min(offset + page_size as usize, trades.len());
        trades[offset..end].to_vec()
    };

    tracing::info!(
        owner = %owner,
        page,
        page_size,
        total_trades,
        returned_trades = paginated.len(),
        "resolved trades by address"
    );

    Ok(Json(TradesByAddressResponse {
        trades: paginated,
        pagination: TradesPagination {
            page,
            page_size,
            total_trades,
            total_pages,
            has_more: u64::from(page) < total_pages,
        },
    }))
}

#[utoipa::path(
    get,
    path = "/v1/trades/tx/{tx_hash}",
    tag = "Trades",
    security(("basicAuth" = [])),
    params(
        ("tx_hash" = String, Path, description = "Transaction hash"),
    ),
    responses(
        (status = 200, description = "Trades from transaction", body = TradesByTxResponse),
        (status = 202, description = "Transaction not yet indexed", body = ApiErrorResponse),
        (status = 401, description = "Unauthorized", body = ApiErrorResponse),
        (status = 429, description = "Rate limited", body = ApiErrorResponse),
        (status = 404, description = "Transaction not found", body = ApiErrorResponse),
        (status = 500, description = "Internal server error", body = ApiErrorResponse),
    )
)]
#[get("/tx/<tx_hash>")]
pub async fn get_trades_by_tx(
    _global: GlobalRateLimit,
    _key: AuthenticatedKey,
    shared_raindex: &State<crate::raindex::SharedRaindexProvider>,
    span: TracingSpan,
    tx_hash: ValidatedFixedBytes,
) -> Result<Json<TradesByTxResponse>, ApiError> {
    async move {
        tracing::info!(tx_hash = ?tx_hash, "request received");
        let raindex = shared_raindex.read().await;
        let ds = RaindexTradesDataSource {
            client: raindex.client(),
        };
        process_get_trades_by_tx(&ds, tx_hash.0).await
    }
    .instrument(span.0)
    .await
}

#[utoipa::path(
    get,
    path = "/v1/trades/{address}",
    tag = "Trades",
    security(("basicAuth" = [])),
    params(
        ("address" = String, Path, description = "Owner address"),
        TradesPaginationParams,
    ),
    responses(
        (status = 200, description = "Paginated list of trades", body = TradesByAddressResponse),
        (status = 400, description = "Bad request", body = ApiErrorResponse),
        (status = 401, description = "Unauthorized", body = ApiErrorResponse),
        (status = 429, description = "Rate limited", body = ApiErrorResponse),
        (status = 500, description = "Internal server error", body = ApiErrorResponse),
    )
)]
#[get("/<address>?<params..>", rank = 2)]
pub async fn get_trades_by_address(
    _global: GlobalRateLimit,
    _key: AuthenticatedKey,
    shared_raindex: &State<crate::raindex::SharedRaindexProvider>,
    span: TracingSpan,
    address: ValidatedAddress,
    params: TradesPaginationParams,
) -> Result<Json<TradesByAddressResponse>, ApiError> {
    async move {
        tracing::info!(address = ?address, params = ?params, "request received");
        let raindex = shared_raindex.read().await;
        let ds = RaindexTradesDataSource {
            client: raindex.client(),
        };
        process_get_trades_by_address(&ds, address.0, params).await
    }
    .instrument(span.0)
    .await
}

pub fn routes() -> Vec<Route> {
    rocket::routes![get_trades_by_tx, get_trades_by_address]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::order::test_fixtures::{mock_order, mock_trade};
    use crate::test_helpers::{basic_auth_header, seed_api_key, TestClientBuilder};
    use rocket::http::{Header, Status};

    struct MockTradesDataSource {
        orders_result: Result<(Vec<RaindexOrder>, u32), ApiError>,
        trades_result: Result<Vec<RaindexTrade>, ApiError>,
        tx_index_state: Result<TxIndexState, ApiError>,
    }

    #[async_trait]
    impl TradesDataSource for MockTradesDataSource {
        async fn get_orders(
            &self,
            _filters: GetOrdersFilters,
            _page: Option<u16>,
            _page_size: Option<u16>,
        ) -> Result<(Vec<RaindexOrder>, u32), ApiError> {
            self.orders_result.clone()
        }

        async fn get_order_trades(
            &self,
            _order: &RaindexOrder,
            _start_time: Option<u64>,
            _end_time: Option<u64>,
        ) -> Result<Vec<RaindexTrade>, ApiError> {
            self.trades_result.clone()
        }

        async fn check_tx_index_state(&self, _tx_hash: B256) -> Result<TxIndexState, ApiError> {
            self.tx_index_state.clone()
        }
    }

    fn tx_hash() -> B256 {
        "0x0000000000000000000000000000000000000000000000000000000000000088"
            .parse()
            .unwrap()
    }

    #[rocket::async_test]
    async fn test_process_get_trades_by_tx_success() {
        let ds = MockTradesDataSource {
            orders_result: Ok((vec![mock_order()], 1)),
            trades_result: Ok(vec![mock_trade()]),
            tx_index_state: Ok(TxIndexState::Indexed),
        };

        let response = process_get_trades_by_tx(&ds, tx_hash())
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.trades.len(), 1);
        assert_eq!(response.block_number, 100);
        assert_eq!(response.timestamp, 1700001000);
        assert_eq!(
            response.sender.to_string(),
            "0x0000000000000000000000000000000000000002"
        );
    }

    #[rocket::async_test]
    async fn test_process_get_trades_by_tx_not_yet_indexed() {
        let ds = MockTradesDataSource {
            orders_result: Ok((vec![mock_order()], 1)),
            trades_result: Ok(vec![]),
            tx_index_state: Ok(TxIndexState::NotYetIndexed),
        };

        let result = process_get_trades_by_tx(&ds, tx_hash()).await;
        assert!(matches!(result, Err(ApiError::NotYetIndexed(_))));
    }

    #[rocket::async_test]
    async fn test_process_get_trades_by_address_success() {
        let ds = MockTradesDataSource {
            orders_result: Ok((vec![mock_order()], 1)),
            trades_result: Ok(vec![mock_trade()]),
            tx_index_state: Ok(TxIndexState::Indexed),
        };

        let response = process_get_trades_by_address(
            &ds,
            "0x0000000000000000000000000000000000000001"
                .parse()
                .unwrap(),
            TradesPaginationParams {
                page: Some(1),
                page_size: Some(20),
                start_time: None,
                end_time: None,
            },
        )
        .await
        .unwrap()
        .into_inner();

        assert_eq!(response.trades.len(), 1);
        assert_eq!(response.pagination.total_trades, 1);
        assert_eq!(response.pagination.total_pages, 1);
        assert!(!response.pagination.has_more);
    }

    #[rocket::async_test]
    async fn test_get_trades_by_tx_401_without_auth() {
        let client = TestClientBuilder::new().build().await;
        let response = client
            .get("/v1/trades/tx/0x0000000000000000000000000000000000000000000000000000000000000088")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    async fn test_get_trades_by_address_401_without_auth() {
        let client = TestClientBuilder::new().build().await;
        let response = client
            .get("/v1/trades/0x0000000000000000000000000000000000000001")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    async fn test_get_trades_by_address_invalid_address_returns_422() {
        let client = TestClientBuilder::new().build().await;
        let (key_id, secret) = seed_api_key(&client).await;
        let header = basic_auth_header(&key_id, &secret);
        let response = client
            .get("/v1/trades/not-an-address")
            .header(Header::new("Authorization", header))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::UnprocessableEntity);
    }
}
