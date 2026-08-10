use serde::{Deserialize, Serialize};

/// Códigos de error estándar de JSON-RPC 2.0. Los usa el lado servidor
/// (daemon Java); en el cliente aún no se construyen respuestas de error.
#[allow(dead_code)]
pub mod codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

#[derive(Debug, Clone, Serialize)]
pub struct RpcRequest {
    pub id: u64,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

impl RpcRequest {
    pub fn encode(&self) -> String {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), serde_json::json!(self.id));
        obj.insert("method".into(), serde_json::json!(self.method));
        if let Some(p) = &self.params {
            obj.insert("params".into(), p.clone());
        }
        obj.insert("jsonrpc".into(), serde_json::json!("2.0"));
        serde_json::Value::Object(obj).to_string()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcErr { pub code: i32, pub message: String }

#[derive(Debug, Clone)]
pub struct RpcResponse {
    pub id: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<RpcErr>,
}

impl RpcResponse {
    pub fn decode(line: &str) -> Result<Self, serde_json::Error> {
        #[derive(Deserialize)]
        struct Raw {
            id: Option<serde_json::Value>,
            result: Option<serde_json::Value>,
            error: Option<RpcErr>,
        }
        let raw: Raw = serde_json::from_str(line)?;
        let id = match raw.id {
            Some(serde_json::Value::Number(n)) => n.as_u64(),
            _ => None,
        };
        Ok(RpcResponse { id, result: raw.result, error: raw.error })
    }

    /// Conveniencia usada por los tests (`rpc_test`).
    #[allow(dead_code)]
    pub fn is_ok(&self) -> bool { self.error.is_none() }

    pub fn unwrap(self) -> Result<serde_json::Value, RpcException> {
        match self.error {
            Some(e) => Err(RpcException { code: e.code, message: e.message }),
            None => Ok(self.result.unwrap_or(serde_json::Value::Null)),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("RpcException({code}): {message}")]
pub struct RpcException { pub code: i32, pub message: String }
