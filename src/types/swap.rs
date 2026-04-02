use crate::types::common::Approval;
use alloy::primitives::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwapQuoteRequest {
    #[schema(value_type = String, example = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")]
    pub input_token: Address,
    #[schema(value_type = String, example = "0x4200000000000000000000000000000000000006")]
    pub output_token: Address,
    #[schema(example = "0.5")]
    pub output_amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwapQuoteResponse {
    #[schema(value_type = String, example = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")]
    pub input_token: Address,
    #[schema(value_type = String, example = "0x4200000000000000000000000000000000000006")]
    pub output_token: Address,
    #[schema(example = "0.5")]
    pub output_amount: String,
    #[schema(example = "0.5")]
    pub estimated_output: String,
    #[schema(example = "1250.75")]
    pub estimated_input: String,
    #[schema(example = "2501.5")]
    pub estimated_io_ratio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwapCalldataRequest {
    #[schema(value_type = String, example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub taker: Address,
    #[schema(value_type = String, example = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")]
    pub input_token: Address,
    #[schema(value_type = String, example = "0x4200000000000000000000000000000000000006")]
    pub output_token: Address,
    #[schema(example = "0.5")]
    pub output_amount: String,
    #[schema(example = "2600")]
    pub maximum_io_ratio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwapCalldataResponse {
    #[schema(value_type = String, example = "0xDEF171Fe48CF0115B1d80b88dc8eAB59176FEe57")]
    pub to: Address,
    #[schema(value_type = String, example = "0xabcdef...")]
    pub data: Bytes,
    #[schema(value_type = String, example = "0x0")]
    pub value: U256,
    #[schema(example = "1250.75")]
    pub estimated_input: String,
    pub approvals: Vec<Approval>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_quote_request_deserializes_camel_case() {
        let json = r#"{
            "inputToken": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "outputToken": "0x4200000000000000000000000000000000000006",
            "outputAmount": "0.5"
        }"#;
        let req: SwapQuoteRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req.input_token,
            "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
                .parse::<Address>()
                .unwrap()
        );
        assert_eq!(req.output_amount, "0.5");
    }

    #[test]
    fn test_swap_quote_request_rejects_missing_fields() {
        let json = r#"{"inputToken": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"}"#;
        let result = serde_json::from_str::<SwapQuoteRequest>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_swap_quote_response_serializes_camel_case() {
        let resp = SwapQuoteResponse {
            input_token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
                .parse()
                .unwrap(),
            output_token: "0x4200000000000000000000000000000000000006"
                .parse()
                .unwrap(),
            output_amount: "0.5".into(),
            estimated_output: "0.5".into(),
            estimated_input: "1250.75".into(),
            estimated_io_ratio: "2501.5".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"inputToken\""));
        assert!(json.contains("\"outputToken\""));
        assert!(json.contains("\"estimatedIoRatio\""));
        assert!(!json.contains("\"input_token\""));
    }

    #[test]
    fn test_swap_quote_response_round_trip() {
        let resp = SwapQuoteResponse {
            input_token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
                .parse()
                .unwrap(),
            output_token: "0x4200000000000000000000000000000000000006"
                .parse()
                .unwrap(),
            output_amount: "0.5".into(),
            estimated_output: "0.5".into(),
            estimated_input: "1250.75".into(),
            estimated_io_ratio: "2501.5".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: SwapQuoteResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.input_token, resp.input_token);
        assert_eq!(deserialized.estimated_io_ratio, "2501.5");
    }

    #[test]
    fn test_swap_calldata_request_deserializes() {
        let json = r#"{
            "taker": "0x1111111111111111111111111111111111111111",
            "inputToken": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "outputToken": "0x4200000000000000000000000000000000000006",
            "outputAmount": "0.5",
            "maximumIoRatio": "2600"
        }"#;
        let req: SwapCalldataRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.maximum_io_ratio, "2600");
        assert_eq!(req.output_amount, "0.5");
    }

    #[test]
    fn test_swap_calldata_request_rejects_missing_taker() {
        let json = r#"{
            "inputToken": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "outputToken": "0x4200000000000000000000000000000000000006",
            "outputAmount": "0.5",
            "maximumIoRatio": "2600"
        }"#;
        assert!(serde_json::from_str::<SwapCalldataRequest>(json).is_err());
    }

    #[test]
    fn test_swap_calldata_response_serializes_with_empty_approvals() {
        let resp = SwapCalldataResponse {
            to: Address::ZERO,
            data: Bytes::new(),
            value: U256::ZERO,
            estimated_input: "100".into(),
            approvals: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"estimatedInput\""));
        assert!(json.contains("\"approvals\":[]"));
    }

    #[test]
    fn test_swap_calldata_response_round_trip() {
        let resp = SwapCalldataResponse {
            to: "0xDEF171Fe48CF0115B1d80b88dc8eAB59176FEe57"
                .parse()
                .unwrap(),
            data: Bytes::new(),
            value: U256::ZERO,
            estimated_input: "1250.75".into(),
            approvals: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: SwapCalldataResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.to, resp.to);
        assert_eq!(deserialized.estimated_input, "1250.75");
    }
}
