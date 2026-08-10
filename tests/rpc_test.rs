use bakeneko::core::daemon::rpc::{RpcErr, RpcRequest, RpcResponse, RpcException, codes};

#[test]
fn request_encodes_jsonrpc_field() {
    let req = RpcRequest { id: 7, method: "ping".into(), params: None };
    let s = req.encode();
    assert!(s.contains("\"id\":7"));
    assert!(s.contains("\"method\":\"ping\""));
    assert!(s.contains("\"jsonrpc\":\"2.0\""));
    assert!(!s.contains('\n'));
}

#[test]
fn response_decode_ok() {
    let line = r#"{"id":7,"result":{"version":"1.0.0","java":"21"},"jsonrpc":"2.0"}"#;
    let r = RpcResponse::decode(line).unwrap();
    assert!(r.is_ok());
    assert_eq!(r.id, Some(7));
    assert_eq!(r.result.unwrap()["version"], "1.0.0");
}

#[test]
fn response_decode_error_and_unwrap() {
    let line = r#"{"id":7,"error":{"code":-32602,"message":"falta source"},"jsonrpc":"2.0"}"#;
    let r = RpcResponse::decode(line).unwrap();
    assert!(!r.is_ok());
    let err = r.unwrap().unwrap_err();
    assert_eq!(err.code, codes::INVALID_PARAMS);
    assert_eq!(err.message, "falta source");
}

#[test]
fn error_omits_id_when_null() {
    let line = r#"{"error":{"code":-32700,"message":"JSON inválido"},"jsonrpc":"2.0"}"#;
    let r = RpcResponse::decode(line).unwrap();
    assert_eq!(r.id, None);
}

#[test]
fn rpc_exception_to_string() {
    let e = RpcException { code: -32601, message: "método desconocido".into() };
    assert!(e.to_string().contains("método desconocido"));
}
