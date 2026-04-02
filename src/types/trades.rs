use crate::types::common::TokenRef;
use alloy::primitives::{Address, FixedBytes};
use rocket::form::FromForm;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, FromForm, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct TradesPaginationParams {
    #[field(name = "page")]
    #[param(example = 1)]
    pub page: Option<u32>,
    #[field(name = "pageSize")]
    #[param(example = 20)]
    pub page_size: Option<u32>,
    #[field(name = "startTime")]
    #[param(example = 1718452800)]
    pub start_time: Option<u64>,
    #[field(name = "endTime")]
    #[param(example = 1718539200)]
    pub end_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradeByAddress {
    #[schema(value_type = String, example = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab")]
    pub tx_hash: FixedBytes<32>,
    #[schema(example = "1000000")]
    pub input_amount: String,
    #[schema(example = "500000")]
    pub output_amount: String,
    pub input_token: TokenRef,
    pub output_token: TokenRef,
    #[schema(value_type = Option<String>)]
    pub order_hash: Option<FixedBytes<32>>,
    #[schema(example = 1718452800)]
    pub timestamp: u64,
    #[schema(example = 12345678)]
    pub block_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradesPagination {
    #[schema(example = 1)]
    pub page: u32,
    #[schema(example = 20)]
    pub page_size: u32,
    #[schema(example = 100)]
    pub total_trades: u64,
    #[schema(example = 5)]
    pub total_pages: u64,
    #[schema(example = true)]
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradesByAddressResponse {
    pub trades: Vec<TradeByAddress>,
    pub pagination: TradesPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradeRequest {
    #[schema(value_type = String, example = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")]
    pub input_token: Address,
    #[schema(value_type = String, example = "0x4200000000000000000000000000000000000006")]
    pub output_token: Address,
    #[schema(example = "1000000")]
    pub maximum_input: String,
    #[schema(example = "0.0006")]
    pub maximum_io_ratio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradeResult {
    #[schema(example = "900000")]
    pub input_amount: String,
    #[schema(example = "500000")]
    pub output_amount: String,
    #[schema(example = "0.00055")]
    pub actual_io_ratio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradeByTxEntry {
    #[schema(value_type = String, example = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab")]
    pub order_hash: FixedBytes<32>,
    #[schema(value_type = String, example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub order_owner: Address,
    pub request: TradeRequest,
    pub result: TradeResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradesTotals {
    #[schema(example = "900000")]
    pub total_input_amount: String,
    #[schema(example = "500000")]
    pub total_output_amount: String,
    #[schema(example = "0.00055")]
    pub average_io_ratio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradesByTxResponse {
    #[schema(value_type = String, example = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab")]
    pub tx_hash: FixedBytes<32>,
    #[schema(example = 12345678)]
    pub block_number: u64,
    #[schema(example = 1718452800)]
    pub timestamp: u64,
    #[schema(value_type = String, example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub sender: Address,
    pub trades: Vec<TradeByTxEntry>,
    pub totals: TradesTotals,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trades_pagination_params_all_optional() {
        let json = r#"{}"#;
        let params: TradesPaginationParams = serde_json::from_str(json).unwrap();
        assert!(params.page.is_none());
        assert!(params.page_size.is_none());
        assert!(params.start_time.is_none());
        assert!(params.end_time.is_none());
    }

    #[test]
    fn test_trades_pagination_params_with_time_range() {
        let json = r#"{"page": 1, "pageSize": 20, "startTime": 1718452800, "endTime": 1718539200}"#;
        let params: TradesPaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.page, Some(1));
        assert_eq!(params.start_time, Some(1718452800));
        assert_eq!(params.end_time, Some(1718539200));
    }

    #[test]
    fn test_trades_pagination_serializes_camel_case() {
        let pagination = TradesPagination {
            page: 1,
            page_size: 20,
            total_trades: 50,
            total_pages: 3,
            has_more: true,
        };
        let json = serde_json::to_string(&pagination).unwrap();
        assert!(json.contains("\"pageSize\""));
        assert!(json.contains("\"totalTrades\""));
        assert!(json.contains("\"totalPages\""));
        assert!(json.contains("\"hasMore\""));
        assert!(!json.contains("\"page_size\""));
    }

    #[test]
    fn test_trades_pagination_round_trip() {
        let original = TradesPagination {
            page: 2,
            page_size: 10,
            total_trades: 30,
            total_pages: 3,
            has_more: false,
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: TradesPagination = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.page, 2);
        assert_eq!(deserialized.total_trades, 30);
        assert!(!deserialized.has_more);
    }

    #[test]
    fn test_trades_totals_serializes_camel_case() {
        let totals = TradesTotals {
            total_input_amount: "900000".into(),
            total_output_amount: "500000".into(),
            average_io_ratio: "0.00055".into(),
        };
        let json = serde_json::to_string(&totals).unwrap();
        assert!(json.contains("\"totalInputAmount\""));
        assert!(json.contains("\"totalOutputAmount\""));
        assert!(json.contains("\"averageIoRatio\""));
    }

    #[test]
    fn test_trade_request_deserializes() {
        let json = r#"{
            "inputToken": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "outputToken": "0x4200000000000000000000000000000000000006",
            "maximumInput": "1000000",
            "maximumIoRatio": "0.0006"
        }"#;
        let req: TradeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.maximum_input, "1000000");
        assert_eq!(req.maximum_io_ratio, "0.0006");
    }

    #[test]
    fn test_trade_result_serializes_camel_case() {
        let result = TradeResult {
            input_amount: "900000".into(),
            output_amount: "500000".into(),
            actual_io_ratio: "0.00055".into(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"inputAmount\""));
        assert!(json.contains("\"actualIoRatio\""));
        assert!(!json.contains("\"input_amount\""));
    }

    #[test]
    fn test_trades_by_address_response_with_empty_trades() {
        let resp = TradesByAddressResponse {
            trades: vec![],
            pagination: TradesPagination {
                page: 1,
                page_size: 20,
                total_trades: 0,
                total_pages: 0,
                has_more: false,
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"trades\":[]"));
    }
}
