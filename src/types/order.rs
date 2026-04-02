use crate::types::common::{Approval, TokenRef};
use alloy::primitives::{Address, Bytes, FixedBytes, U256};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PeriodUnit {
    Days,
    Hours,
    Minutes,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeployDcaOrderRequest {
    #[schema(value_type = String, example = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")]
    pub input_token: Address,
    #[schema(value_type = String, example = "0x4200000000000000000000000000000000000006")]
    pub output_token: Address,
    #[schema(example = "1000000")]
    pub budget_amount: String,
    #[schema(example = 4)]
    pub period: u32,
    #[schema(example = "hours")]
    pub period_unit: PeriodUnit,
    #[schema(example = "0.0005")]
    pub start_io: String,
    #[schema(example = "0.0003")]
    pub floor_io: String,
    #[schema(value_type = Option<String>)]
    pub input_vault_id: Option<U256>,
    #[schema(value_type = Option<String>)]
    pub output_vault_id: Option<U256>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploySolverOrderRequest {
    #[schema(value_type = String, example = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")]
    pub input_token: Address,
    #[schema(value_type = String, example = "0x4200000000000000000000000000000000000006")]
    pub output_token: Address,
    #[schema(example = "1000000")]
    pub amount: String,
    #[schema(example = "0.0005")]
    pub io_ratio: String,
    #[schema(value_type = Option<String>)]
    pub input_vault_id: Option<U256>,
    #[schema(value_type = Option<String>)]
    pub output_vault_id: Option<U256>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeployOrderResponse {
    #[schema(value_type = String, example = "0xDEF171Fe48CF0115B1d80b88dc8eAB59176FEe57")]
    pub to: Address,
    #[schema(value_type = String, example = "0xabcdef...")]
    pub data: Bytes,
    #[schema(value_type = String, example = "0x0")]
    pub value: U256,
    pub approvals: Vec<Approval>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderRequest {
    #[schema(value_type = String, example = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab")]
    pub order_hash: FixedBytes<32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelTransaction {
    #[schema(value_type = String, example = "0xDEF171Fe48CF0115B1d80b88dc8eAB59176FEe57")]
    pub to: Address,
    #[schema(value_type = String, example = "0xabcdef...")]
    pub data: Bytes,
    #[schema(value_type = String, example = "0x0")]
    pub value: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenReturn {
    #[schema(value_type = String, example = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")]
    pub token: Address,
    #[schema(example = "USDC")]
    pub symbol: String,
    #[schema(example = "1000000")]
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelSummary {
    #[schema(example = 2)]
    pub vaults_to_withdraw: u32,
    pub tokens_returned: Vec<TokenReturn>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderResponse {
    pub transactions: Vec<CancelTransaction>,
    pub summary: CancelSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Dca,
    Solver,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetailsInfo {
    #[serde(rename = "type")]
    #[schema(example = "dca")]
    pub type_: OrderType,
    #[schema(example = "0.0005")]
    pub io_ratio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderTradeEntry {
    #[schema(example = "trade-1")]
    pub id: String,
    #[schema(value_type = String, example = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab")]
    pub tx_hash: FixedBytes<32>,
    #[schema(example = "1000000")]
    pub input_amount: String,
    #[schema(example = "500000")]
    pub output_amount: String,
    #[schema(example = 1718452800)]
    pub timestamp: u64,
    #[schema(value_type = String, example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub sender: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetail {
    #[schema(value_type = String, example = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab")]
    pub order_hash: FixedBytes<32>,
    #[schema(value_type = String, example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub owner: Address,
    pub order_details: OrderDetailsInfo,
    pub input_token: TokenRef,
    pub output_token: TokenRef,
    #[schema(value_type = String, example = "0x1")]
    pub input_vault_id: U256,
    #[schema(value_type = String, example = "0x2")]
    pub output_vault_id: U256,
    #[schema(example = "1000000")]
    pub input_vault_balance: String,
    #[schema(example = "500000")]
    pub output_vault_balance: String,
    #[schema(example = "0.0005")]
    pub io_ratio: String,
    #[schema(example = 1718452800)]
    pub created_at: u64,
    #[schema(value_type = String, example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub orderbook_id: Address,
    pub trades: Vec<OrderTradeEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_period_unit_variants() {
        let variants = [
            ("\"minutes\"", PeriodUnit::Minutes),
            ("\"hours\"", PeriodUnit::Hours),
            ("\"days\"", PeriodUnit::Days),
        ];
        for (json, expected) in variants {
            let parsed: PeriodUnit = serde_json::from_str(json).unwrap();
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn test_period_unit_rejects_invalid() {
        let result = serde_json::from_str::<PeriodUnit>("\"seconds\"");
        assert!(result.is_err());
        let result = serde_json::from_str::<PeriodUnit>("\"weeks\"");
        assert!(result.is_err());
        let result = serde_json::from_str::<PeriodUnit>("\"months\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_order_details_info_type_rename() {
        let info = OrderDetailsInfo {
            type_: OrderType::Dca,
            io_ratio: "0.0005".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"type\":\"dca\""));
        assert!(!json.contains("\"type_\""));
    }

    #[test]
    fn test_order_type_serializes_lowercase() {
        let dca = serde_json::to_string(&OrderType::Dca).unwrap();
        let solver = serde_json::to_string(&OrderType::Solver).unwrap();
        assert_eq!(dca, "\"dca\"");
        assert_eq!(solver, "\"solver\"");
    }

    #[test]
    fn test_order_type_rejects_uppercase() {
        assert!(serde_json::from_str::<OrderType>("\"DCA\"").is_err());
        assert!(serde_json::from_str::<OrderType>("\"Solver\"").is_err());
    }

    #[test]
    fn test_deploy_solver_order_request_deserializes() {
        let json = r#"{
            "inputToken": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "outputToken": "0x4200000000000000000000000000000000000006",
            "amount": "1000000",
            "ioRatio": "0.0005"
        }"#;
        let req: DeploySolverOrderRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount, "1000000");
        assert_eq!(req.io_ratio, "0.0005");
        assert!(req.input_vault_id.is_none());
        assert!(req.output_vault_id.is_none());
    }

    #[test]
    fn test_deploy_solver_order_request_with_vault_ids() {
        let json = r#"{
            "inputToken": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "outputToken": "0x4200000000000000000000000000000000000006",
            "amount": "1000000",
            "ioRatio": "0.0005",
            "inputVaultId": "0x1",
            "outputVaultId": "0x2"
        }"#;
        let req: DeploySolverOrderRequest = serde_json::from_str(json).unwrap();
        assert!(req.input_vault_id.is_some());
        assert!(req.output_vault_id.is_some());
    }

    #[test]
    fn test_deploy_dca_order_request_deserializes() {
        let json = r#"{
            "inputToken": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "outputToken": "0x4200000000000000000000000000000000000006",
            "budgetAmount": "1000",
            "period": 4,
            "periodUnit": "hours",
            "startIo": "0.0005",
            "floorIo": "0.0003"
        }"#;
        let req: DeployDcaOrderRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.budget_amount, "1000");
        assert_eq!(req.period, 4);
        assert_eq!(req.period_unit, PeriodUnit::Hours);
        assert_eq!(req.start_io, "0.0005");
        assert_eq!(req.floor_io, "0.0003");
    }

    #[test]
    fn test_deploy_dca_rejects_missing_period_unit() {
        let json = r#"{
            "inputToken": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "outputToken": "0x4200000000000000000000000000000000000006",
            "budgetAmount": "1000",
            "period": 4,
            "startIo": "0.0005",
            "floorIo": "0.0003"
        }"#;
        assert!(serde_json::from_str::<DeployDcaOrderRequest>(json).is_err());
    }

    #[test]
    fn test_cancel_order_request_deserializes() {
        let json = r#"{
            "orderHash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab"
        }"#;
        let req: CancelOrderRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            format!("{}", req.order_hash),
            "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab"
        );
    }

    #[test]
    fn test_cancel_order_request_rejects_short_hash() {
        let json = r#"{"orderHash": "0xabcdef"}"#;
        assert!(serde_json::from_str::<CancelOrderRequest>(json).is_err());
    }

    #[test]
    fn test_deploy_order_response_serializes_camel_case() {
        let resp = DeployOrderResponse {
            to: Address::ZERO,
            data: Bytes::new(),
            value: U256::ZERO,
            approvals: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"approvals\""));
        assert!(!json.contains("\"to_\""));
    }

    #[test]
    fn test_cancel_summary_serializes_camel_case() {
        let summary = CancelSummary {
            vaults_to_withdraw: 2,
            tokens_returned: vec![],
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"vaultsToWithdraw\""));
        assert!(json.contains("\"tokensReturned\""));
        assert!(!json.contains("\"vaults_to_withdraw\""));
    }

    #[test]
    fn test_order_trade_entry_serializes_camel_case() {
        let entry = OrderTradeEntry {
            id: "trade-1".into(),
            tx_hash: FixedBytes::ZERO,
            input_amount: "1000".into(),
            output_amount: "500".into(),
            timestamp: 1718452800,
            sender: Address::ZERO,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"txHash\""));
        assert!(json.contains("\"inputAmount\""));
        assert!(json.contains("\"outputAmount\""));
        assert!(!json.contains("\"tx_hash\""));
    }
}
