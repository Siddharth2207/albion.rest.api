//! GET /v1/schemas: query subgraph for offchain asset receipt vaults and decode schema information.

use crate::auth::AuthenticatedKey;
use crate::error::{ApiError, ApiErrorResponse};
use crate::fairings::{GlobalRateLimit, TracingSpan};
use crate::types::schemas::{
    GetSchemasRequest, OA_SCHEMA_MAGIC, ReceiptVaultInformation, SchemaQueryResponse,
    SchemasSubgraphResponse,
};
use alloy::primitives::hex;
use rocket::fairing::AdHoc;
use rocket::serde::json::Json;
use rocket::{Route, State};
use serde::{Deserialize, Serialize};
use serde_cbor::Value as CborValue;
use std::time::Duration;
use tracing::Instrument;

const SCHEMAS_SUBGRAPH_URL: &str = "https://api.goldsky.com/api/public/project_cm153vmqi5gke01vy66p4ftzf/subgraphs/sft-offchainassetvaulttest-base/1.0.5/gn";
const SUBGRAPH_TIMEOUT_SECS: u64 = 30;

/// 8-byte magic prefix before the CBOR payload in receipt vault information (same as Rain Meta Document v1 prefix skip).
const INFORMATION_CBOR_SKIP_BYTES: usize = 8;

/// In JS, information.slice(18) skips "0x" (2 chars) + 8 bytes as hex (16 chars) = 18.
/// So we decode hex and then skip the first 8 bytes before CBOR decoding.
fn information_bytes_to_cbor_payload(hex_str: &str) -> Result<Vec<u8>, ApiError> {
    let s = hex_str.trim_start_matches("0x");
    let bytes = hex::decode(s).map_err(|e| ApiError::BadRequest(format!("invalid information hex: {}", e)))?;
    if bytes.len() <= INFORMATION_CBOR_SKIP_BYTES {
        return Err(ApiError::BadRequest(
            "information too short to contain CBOR".into(),
        ));
    }
    Ok(bytes[INFORMATION_CBOR_SKIP_BYTES..].to_vec())
}

/// Decode CBOR payload into a pair of values. The payload is a sequence of CBOR values
/// (there may be trailing data). We read the first two values. Supports:
/// - First value is an array of ≥2 elements → use first two elements as the pair.
/// - Otherwise → use first and second decoded values as the pair (e.g. two maps).
fn cbor_decode_two_maps(payload: &[u8]) -> Result<(CborValue, CborValue), ApiError> {
    let mut deserializer = serde_cbor::Deserializer::from_slice(payload);
    let first: CborValue =
        serde::Deserialize::deserialize(&mut deserializer).map_err(|e| {
            ApiError::Internal(format!("CBOR first value decode failed: {}", e))
        })?;
    let second: CborValue =
        serde::Deserialize::deserialize(&mut deserializer).map_err(|e| {
            ApiError::Internal(format!("CBOR second value decode failed: {}", e))
        })?;

    match &first {
        CborValue::Array(arr) if arr.len() >= 2 => {
            let a = arr[0].clone();
            let b = arr[1].clone();
            Ok((a, b))
        }
        _ => Ok((first, second)),
    }
}

fn cbor_map_get(map: &std::collections::BTreeMap<CborValue, CborValue>, key: i64) -> Option<&CborValue> {
    let k = CborValue::Integer(key as i128);
    map.get(&k)
}

fn cbor_map_get_u64(value: &CborValue) -> Option<u64> {
    match value {
        CborValue::Integer(n) => {
            if *n >= 0 {
                u64::try_from(*n).ok()
            } else {
                None
            }
        }
        _ => None,
    }
}

fn cbor_value_as_bytes(value: &CborValue) -> Option<Vec<u8>> {
    match value {
        CborValue::Bytes(b) => Some(b.clone()),
        CborValue::Text(s) => Some(s.as_bytes().to_vec()),
        _ => None,
    }
}

fn cbor_value_to_string(value: &CborValue) -> Option<String> {
    match value {
        CborValue::Text(s) => Some(s.clone()),
        CborValue::Bytes(b) => Some(alloy::primitives::hex::encode_prefixed(b)),
        _ => None,
    }
}

/// Decode one receipt vault information entry into zero or more schema responses.
/// If the CBOR has the expected shape (array of ≥2 maps, first has payload key 0 and magic key 1,
/// second has schema hash at key 0) we return a schema. When magic doesn't match OA_SCHEMA_MAGIC
/// we still decode and log the actual magic so you can set the constant correctly.
fn decode_receipt_vault_information(
    info: &ReceiptVaultInformation,
) -> Result<Vec<SchemaQueryResponse>, ApiError> {
    let information = match &info.information {
        Some(s) if !s.is_empty() => s.as_str(),
        _ => return Ok(vec![]),
    };

    let cbor_payload = match information_bytes_to_cbor_payload(information) {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!(error = %e, "information hex/cbor skip failed");
            return Err(e);
        }
    };

    let (first_val, second_val) = match cbor_decode_two_maps(&cbor_payload) {
        Ok(pair) => pair,
        Err(e) => {
            tracing::info!(error = %e, "schemas: CBOR decode failed (not array or two maps)");
            return Err(e);
        }
    };

    let first_map = match &first_val {
        CborValue::Map(m) => m,
        _ => {
            tracing::debug!("first CBOR item is not a map");
            return Ok(vec![]);
        }
    };

    let magic_val = match cbor_map_get(first_map, 1) {
        Some(v) => v,
        None => {
            tracing::debug!("first map has no key 1 (magic)");
            return Ok(vec![]);
        }
    };
    let magic_u64 = cbor_map_get_u64(magic_val);
    if magic_u64 != Some(OA_SCHEMA_MAGIC) {
        tracing::debug!(
            actual_magic = ?magic_u64,
            expected = OA_SCHEMA_MAGIC,
            "magic mismatch (set OA_SCHEMA_MAGIC in src/types/schemas.rs to actual_magic to filter)"
        );
        // Still decode and return so you get a result; you can tighten later with correct OA_SCHEMA_MAGIC
    }

    let second_map = match &second_val {
        CborValue::Map(m) => m,
        _ => {
            tracing::debug!("second CBOR item is not a map");
            return Ok(vec![]);
        }
    };
    let schema_hash_value = cbor_map_get(second_map, 0);
    let schema_hash: Option<String> = schema_hash_value.and_then(cbor_value_to_string);
    if let Some(ref h) = schema_hash {
        if h.contains(',') {
            tracing::debug!(hash = %h, "schema hash contains comma, skipping");
            return Ok(vec![]);
        }
    }

    let payload_value = cbor_map_get(first_map, 0);
    let payload_bytes = match payload_value.and_then(cbor_value_as_bytes) {
        Some(b) => b,
        None => {
            tracing::debug!("first map has no payload bytes/text at key 0");
            return Ok(vec![]);
        }
    };

    let structure: serde_json::Value = serde_json::from_slice(payload_bytes.as_slice())
        .unwrap_or_else(|_| serde_json::json!({}));

    let display_name = structure
        .get("displayName")
        .or_else(|| structure.get("display_name"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| structure.get("name").and_then(|v| v.as_str()).map(String::from));

    let display_name = display_name.or_else(|| schema_hash.as_ref().map(|h| h[..h.len().min(18)].to_string()));

    if schema_hash.is_none() {
        tracing::debug!("no schema hash from second CBOR map");
        return Ok(vec![]);
    }

    Ok(vec![SchemaQueryResponse {
        display_name,
        timestamp: info.timestamp.clone(),
        id: info.id.clone(),
        hash: schema_hash,
        structure,
    }])
}

#[derive(Debug, Serialize, Deserialize)]
struct GraphqlBody {
    query: String,
    variables: Option<serde_json::Value>,
}

#[utoipa::path(
    post,
    path = "/v1/schemas",
    tag = "Schemas",
    request_body = GetSchemasRequest,
    security(("basicAuth" = [])),
    responses(
        (status = 200, description = "List of decoded schemas for the given vault", body = Vec<SchemaQueryResponse>),
        (status = 400, description = "Bad request (e.g. invalid vault id)", body = ApiErrorResponse),
        (status = 401, description = "Unauthorized", body = ApiErrorResponse),
        (status = 429, description = "Rate limited", body = ApiErrorResponse),
        (status = 500, description = "Internal server error", body = ApiErrorResponse),
    )
)]
#[post("/", data = "<request>")]
pub async fn post_schemas(
    _global: GlobalRateLimit,
    _key: AuthenticatedKey,
    span: TracingSpan,
    state: &State<SchemasConfig>,
    request: Json<GetSchemasRequest>,
) -> Result<Json<Vec<SchemaQueryResponse>>, ApiError> {
    let vault_id = request.vault_id.trim().to_lowercase();
    if vault_id.is_empty() {
        return Err(ApiError::BadRequest("vaultId is required".into()));
    }
    async move {
        const QUERY: &str = r#"
query GetOffchainAssetReceiptVaultsByVaultId($vaultId: ID!) {
  offchainAssetReceiptVaults(where: { id: $vaultId }) {
    id
    address
    receiptVaultInformations {
      id
      payload
      information
      timestamp
      emitter {
        address
      }
    }
  }
}
"#;
        let body = GraphqlBody {
            query: QUERY.to_string(),
            variables: Some(serde_json::json!({ "vaultId": vault_id })),
        };
        let res = state
            .client
            .post(&state.url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "schemas subgraph request failed");
                ApiError::Internal(format!("subgraph request failed: {}", e))
            })?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            tracing::warn!(status = %status, body = %text, "schemas subgraph non-2xx");
            return Err(ApiError::Internal(format!(
                "subgraph returned {}: {}",
                status,
                text.chars().take(200).collect::<String>()
            )));
        }

        let subgraph: SchemasSubgraphResponse = res.json().await.map_err(|e| {
            tracing::warn!(error = %e, "schemas subgraph response parse failed");
            ApiError::Internal(format!("subgraph response parse failed: {}", e))
        })?;

        let vaults = subgraph
            .data
            .and_then(|d| d.offchain_asset_receipt_vaults)
            .unwrap_or_default();

        let num_vaults = vaults.len();
        let num_infos: usize = vaults.iter().map(|v| v.receipt_vault_informations.len()).sum();
        tracing::info!(
            vault_id = %vault_id,
            vaults_returned = num_vaults,
            receipt_vault_informations_total = num_infos,
            "subgraph response"
        );

        if num_vaults == 0 {
            tracing::warn!(
                vault_id = %vault_id,
                "subgraph returned 0 vaults; check id format (e.g. lowercase 0x...) or subgraph schema"
            );
        }
        if num_vaults > 0 && num_infos == 0 {
            tracing::warn!(vault_id = %vault_id, "vault(s) returned but no receiptVaultInformations");
        }

        let mut out = Vec::new();
        for vault in &vaults {
            for info in &vault.receipt_vault_informations {
                match decode_receipt_vault_information(info) {
                    Ok(schemas) => out.extend(schemas),
                    Err(e) => {
                        tracing::debug!(vault_id = %vault.id, error = %e, "skip decoding one receipt vault information");
                    }
                }
            }
        }

        Ok(Json(out))
    }
    .instrument(span.0)
    .await
}

pub struct SchemasConfig {
    pub url: String,
    pub client: reqwest::Client,
}

impl Default for SchemasConfig {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(SUBGRAPH_TIMEOUT_SECS))
            .build()
            .expect("reqwest client");
        Self {
            url: SCHEMAS_SUBGRAPH_URL.to_string(),
            client,
        }
    }
}

pub(crate) fn fairing() -> AdHoc {
    AdHoc::on_ignite("Schemas Config", |rocket| async {
        if rocket.state::<SchemasConfig>().is_some() {
            tracing::info!("SchemasConfig already managed; skipping default initialization");
            rocket
        } else {
            tracing::info!(url = %SCHEMAS_SUBGRAPH_URL, "initializing default SchemasConfig");
            rocket.manage(SchemasConfig::default())
        }
    })
}

pub fn routes() -> Vec<Route> {
    rocket::routes![post_schemas].into_iter().collect()
}
