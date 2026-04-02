use alloy::primitives::{Address, Bytes, FixedBytes};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenRef {
    #[schema(value_type = String, example = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")]
    pub address: Address,
    #[schema(example = "USDC")]
    pub symbol: String,
    #[schema(example = 6)]
    pub decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    #[schema(value_type = String, example = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")]
    pub token: Address,
    #[schema(value_type = String, example = "0x1234567890abcdef1234567890abcdef12345678")]
    pub spender: Address,
    #[schema(example = "1000000")]
    pub amount: String,
    #[schema(example = "USDC")]
    pub symbol: String,
    #[schema(value_type = String, example = "0xabcdef...")]
    pub approval_data: Bytes,
}

#[derive(Debug)]
pub struct ValidatedAddress(pub Address);

impl<'a> rocket::request::FromParam<'a> for ValidatedAddress {
    type Error = &'a str;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        param.parse::<Address>().map(ValidatedAddress).map_err(|e| {
            tracing::warn!(input = %param, error = %e, "invalid address parameter");
            param
        })
    }
}

#[derive(Debug)]
pub struct ValidatedFixedBytes(pub FixedBytes<32>);

impl<'a> rocket::request::FromParam<'a> for ValidatedFixedBytes {
    type Error = &'a str;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        param
            .parse::<FixedBytes<32>>()
            .map(ValidatedFixedBytes)
            .map_err(|e| {
                tracing::warn!(input = %param, error = %e, "invalid fixed bytes parameter");
                param
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::request::FromParam;

    #[test]
    fn test_path_address_valid() {
        let result = ValidatedAddress::from_param("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_address_rejects_garbage() {
        let result = ValidatedAddress::from_param("not-an-address");
        assert!(result.is_err());
    }

    #[test]
    fn test_path_address_rejects_wrong_length() {
        let result = ValidatedAddress::from_param("0x833589");
        assert!(result.is_err());
    }

    #[test]
    fn test_path_address_rejects_non_hex() {
        let result = ValidatedAddress::from_param("0xZZZZ89fCD6eDb6E08f4c7C32D4f71b54bdA02913");
        assert!(result.is_err());
    }

    #[test]
    fn test_path_fixed_bytes_valid() {
        let result = ValidatedFixedBytes::from_param(
            "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_fixed_bytes_rejects_garbage() {
        let result = ValidatedFixedBytes::from_param("not-a-hash");
        assert!(result.is_err());
    }

    #[test]
    fn test_path_fixed_bytes_rejects_wrong_length() {
        let result = ValidatedFixedBytes::from_param("0xabcdef");
        assert!(result.is_err());
    }

    #[test]
    fn test_path_fixed_bytes_rejects_non_hex() {
        let result = ValidatedFixedBytes::from_param(
            "0xZZZZef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        );
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // TokenRef – serde
    // -----------------------------------------------------------------------

    #[test]
    fn test_token_ref_serializes_camel_case() {
        let token = TokenRef {
            address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
                .parse()
                .unwrap(),
            symbol: "USDC".into(),
            decimals: 6,
        };
        let json = serde_json::to_string(&token).unwrap();
        // camelCase is already just single words, but ensure no snake_case
        assert!(json.contains("\"address\""));
        assert!(json.contains("\"symbol\""));
        assert!(json.contains("\"decimals\""));
        assert!(json.contains("\"USDC\""));
        assert!(json.contains("6"));
    }

    #[test]
    fn test_token_ref_round_trip() {
        let original = TokenRef {
            address: "0x4200000000000000000000000000000000000006"
                .parse()
                .unwrap(),
            symbol: "WETH".into(),
            decimals: 18,
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: TokenRef = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.address, original.address);
        assert_eq!(deserialized.symbol, "WETH");
        assert_eq!(deserialized.decimals, 18);
    }

    #[test]
    fn test_token_ref_rejects_missing_fields() {
        let json = r#"{"address":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"}"#;
        assert!(serde_json::from_str::<TokenRef>(json).is_err());
    }

    // -----------------------------------------------------------------------
    // Approval – serde
    // -----------------------------------------------------------------------

    #[test]
    fn test_approval_serializes_camel_case() {
        let approval = Approval {
            token: Address::ZERO,
            spender: Address::ZERO,
            amount: "1000000".into(),
            symbol: "USDC".into(),
            approval_data: Bytes::new(),
        };
        let json = serde_json::to_string(&approval).unwrap();
        assert!(json.contains("\"approvalData\""));
        assert!(!json.contains("\"approval_data\""));
    }

    #[test]
    fn test_approval_round_trip() {
        let original = Approval {
            token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
                .parse()
                .unwrap(),
            spender: "0x4200000000000000000000000000000000000006"
                .parse()
                .unwrap(),
            amount: "500".into(),
            symbol: "USDC".into(),
            approval_data: Bytes::from(vec![0x01, 0x02]),
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Approval = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.token, original.token);
        assert_eq!(deserialized.spender, original.spender);
        assert_eq!(deserialized.amount, "500");
        assert_eq!(deserialized.symbol, "USDC");
    }

    // -----------------------------------------------------------------------
    // ValidatedAddress – edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_path_address_accepts_lowercase() {
        let result = ValidatedAddress::from_param("0x833589fcd6edb6e08f4c7c32d4f71b54bda02913");
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_address_accepts_checksummed() {
        let result = ValidatedAddress::from_param("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_address_rejects_empty_string() {
        let result = ValidatedAddress::from_param("");
        assert!(result.is_err());
    }

    #[test]
    fn test_path_fixed_bytes_rejects_empty_string() {
        let result = ValidatedFixedBytes::from_param("");
        assert!(result.is_err());
    }
}
