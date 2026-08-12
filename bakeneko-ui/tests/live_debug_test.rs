// Debug: qué recibe exactamente el cliente del daemon real (socket directo).
use bakeneko::core::daemon::rpc::{RpcRequest, RpcResponse};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
#[ignore = "requiere daemon Java corriendo activamente en socket Unix"]
async fn live_daemon_catalog_count() {


    let sock = std::env::var("XDG_RUNTIME_DIR")
        .map(|r| format!("{r}/bakeneko/daemon.sock"))
        .unwrap_or_else(|_| "/run/user/1000/bakeneko/daemon.sock".into());
    let stream = tokio::net::UnixStream::connect(&sock).await.expect("connect");
    let (read_half, mut write_half) = stream.into_split();
    let req = RpcRequest {
        id: 1,
        method: "catalog.list".into(),
        params: Some(serde_json::json!({"source": "LUNARANIME", "offset": 0})),
    };
    write_half
        .write_all(format!("{}\n", req.encode()).as_bytes())
        .await
        .unwrap();
    let mut lines = BufReader::new(read_half).lines();
    let line = lines.next_line().await.unwrap().expect("response line");
    eprintln!("RAW RESPONSE BYTES = {}", line.len());
    let resp = RpcResponse::decode(&line).unwrap();
    let value = resp.unwrap().unwrap();
    let arr = value.as_array().unwrap();
    eprintln!("DAEMON SENT {} ITEMS", arr.len());

    // Ahora la deserialización como en DaemonClient:
    let parsed: Result<Vec<bakeneko::core::models::Manga>, _> =
        serde_json::from_value(value.clone());
    match &parsed {
        Ok(list) => eprintln!("DESERIALIZED {} MANGA OK", list.len()),
        Err(e) => {
            eprintln!("DESERIALIZE FAILED: {e}");
            // Encuentra el primer item que no deserializa
            for (i, item) in arr.iter().enumerate() {
                if serde_json::from_value::<bakeneko::core::models::Manga>(item.clone()).is_err() {
                    let e2 = serde_json::from_value::<bakeneko::core::models::Manga>(item.clone()).unwrap_err();
                    eprintln!("FIRST BAD ITEM index {i}: {e2}\nJSON: {}", serde_json::to_string(item).unwrap());
                    break;
                }
            }
        }
    }
    assert!(parsed.is_ok());
}
