use crate::types::common::TokenRef;
use alloy::primitives::{Address, FixedBytes};
use rocket::form::{FromForm, FromFormField};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, FromForm, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct OrdersPaginationParams {
    #[field(name = "page")]
    #[param(example = 1)]
    pub page: Option<u16>,
    #[field(name = "pageSize")]
    #[param(example = 20)]
    pub page_size: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromFormField, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum OrderSide {
    Input,
    Output,
}

#[derive(Debug, Clone, FromForm, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct OrdersByTokenParams {
    #[field(name = "side")]
    pub side: Option<OrderSide>,
    #[field(name = "page")]
    #[param(example = 1)]
    pub page: Option<u16>,
    #[field(name = "pageSize")]
    #[param(example = 20)]
    pub page_size: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderSummary {
    #[schema(value_type = String, example = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab")]
    pub order_hash: FixedBytes<32>,
    #[schema(value_type = String, example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub owner: Address,
    pub input_token: TokenRef,
    pub output_token: TokenRef,
    #[schema(example = "500000")]
    pub output_vault_balance: String,
    #[schema(example = "0.0005")]
    pub io_ratio: String,
    #[schema(example = 1718452800)]
    pub created_at: u64,
    #[schema(value_type = String, example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub orderbook_id: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrdersPagination {
    #[schema(example = 1)]
    pub page: u32,
    #[schema(example = 20)]
    pub page_size: u32,
    #[schema(example = 100)]
    pub total_orders: u64,
    #[schema(example = 5)]
    pub total_pages: u64,
    #[schema(example = true)]
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrdersListResponse {
    pub orders: Vec<OrderSummary>,
    pub pagination: OrdersPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderByTxEntry {
    #[schema(value_type = String, example = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab")]
    pub order_hash: FixedBytes<32>,
    #[schema(value_type = String, example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub owner: Address,
    #[schema(value_type = String, example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub orderbook_id: Address,
    pub input_token: TokenRef,
    pub output_token: TokenRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrdersByTxResponse {
    #[schema(value_type = String, example = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab")]
    pub tx_hash: FixedBytes<32>,
    #[schema(example = 12345678)]
    pub block_number: u64,
    #[schema(example = 1718452800)]
    pub timestamp: u64,
    pub orders: Vec<OrderByTxEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_side_deserializes_input() {
        let json = r#""input""#;
        let side: OrderSide = serde_json::from_str(json).unwrap();
        assert!(matches!(side, OrderSide::Input));
    }

    #[test]
    fn test_order_side_deserializes_output() {
        let json = r#""output""#;
        let side: OrderSide = serde_json::from_str(json).unwrap();
        assert!(matches!(side, OrderSide::Output));
    }

    #[test]
    fn test_order_side_rejects_invalid() {
        assert!(serde_json::from_str::<OrderSide>(r#""buy""#).is_err());
        assert!(serde_json::from_str::<OrderSide>(r#""sell""#).is_err());
        assert!(serde_json::from_str::<OrderSide>(r#""both""#).is_err());
    }

    #[test]
    fn test_orders_pagination_serializes_camel_case() {
        let pagination = OrdersPagination {
            page: 1,
            page_size: 20,
            total_orders: 100,
            total_pages: 5,
            has_more: true,
        };
        let json = serde_json::to_string(&pagination).unwrap();
        assert!(json.contains("\"pageSize\""));
        assert!(json.contains("\"totalOrders\""));
        assert!(json.contains("\"totalPages\""));
        assert!(json.contains("\"hasMore\""));
        assert!(!json.contains("\"page_size\""));
    }

    #[test]
    fn test_orders_pagination_round_trip() {
        let original = OrdersPagination {
            page: 3,
            page_size: 50,
            total_orders: 200,
            total_pages: 4,
            has_more: false,
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: OrdersPagination = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.page, 3);
        assert_eq!(deserialized.page_size, 50);
        assert_eq!(deserialized.total_orders, 200);
        assert!(!deserialized.has_more);
    }

    #[test]
    fn test_orders_list_response_with_empty_orders() {
        let resp = OrdersListResponse {
            orders: vec![],
            pagination: OrdersPagination {
                page: 1,
                page_size: 20,
                total_orders: 0,
                total_pages: 0,
                has_more: false,
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"orders\":[]"));
    }

    #[test]
    fn test_orders_pagination_params_page_is_optional() {
        let json = r#"{}"#;
        let params: OrdersPaginationParams = serde_json::from_str(json).unwrap();
        assert!(params.page.is_none());
        assert!(params.page_size.is_none());
    }

    #[test]
    fn test_orders_pagination_params_with_values() {
        let json = r#"{"page": 2, "pageSize": 50}"#;
        let params: OrdersPaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.page, Some(2));
        assert_eq!(params.page_size, Some(50));
    }
}
