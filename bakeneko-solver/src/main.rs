use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::{WebContext, WebViewBuilder};

#[derive(Serialize, Deserialize)]
struct SocketRequest {
    #[serde(default)]
    id: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    ping: bool,
}

#[derive(Serialize, Deserialize)]
struct SocketResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pong: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct IpcMessage {
    id: String,
    #[serde(default)]
    status: u16,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: String,
}

enum UserEvent {
    Fetch {
        id: String,
        url: String,
        responder: Sender<SocketResponse>,
    },
    IpcResult(String),
}

fn get_data_dir() -> PathBuf {
    let xdg_data = env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.local/share", home)
    });
    PathBuf::from(xdg_data).join("bakeneko")
}

fn get_solver_socket_path() -> PathBuf {
    let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        let uid = unsafe { libc::getuid() };
        format!("/tmp/bakeneko-{}", uid)
    });
    PathBuf::from(runtime_dir).join("bakeneko").join("solver.sock")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env::set_var("NO_AT_BRIDGE", "1");
    env::set_var("GTK_A11Y", "none");
    env::set_var("GST_DEBUG", "0");
    env::set_var("PULSE_SERVER", "");
    env::set_var("PIPEWIRE_REMOTE", "");

    let sock_path = get_solver_socket_path();
    if let Some(parent) = sock_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if sock_path.exists() {
        let _ = fs::remove_file(&sock_path);
    }

    let base_url = "https://mangadot.net";

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // Ventana 1x1 fuera de pantalla para que GTK inicialice el WebKit sin ventana visible en el escritorio
    let window = WindowBuilder::new()
        .with_title("Bakeneko Solver Engine")
        .with_visible(true)
        .with_decorations(false)
        .with_inner_size(tao::dpi::LogicalSize::new(1.0, 1.0))
        .with_position(tao::dpi::LogicalPosition::new(-10000.0, -10000.0))
        .build(&event_loop)?;

    let bakeneko_dir = get_data_dir();
    let profile_dir = bakeneko_dir.join("solver_profile");
    let _ = fs::create_dir_all(&profile_dir);
    let mut web_context = WebContext::new(Some(profile_dir));

    let proxy_ipc = proxy.clone();
    let webview = WebViewBuilder::with_web_context(&mut web_context)
        .with_url(base_url)
        .with_ipc_handler(move |req| {
            let _ = proxy_ipc.send_event(UserEvent::IpcResult(req.body().clone()));
        })
        .build(&window)?;

    // Iniciar servidor Unix Domain Socket en un hilo dedicado
    let listener = UnixListener::bind(&sock_path)?;
    let proxy_server = proxy.clone();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let proxy_inner = proxy_server.clone();
            thread::spawn(move || {
                let mut reader = BufReader::new(&stream);
                let mut writer = &stream;
                let mut line = String::new();
                while let Ok(n) = reader.read_line(&mut line) {
                    if n == 0 { break; }
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        line.clear();
                        continue;
                    }
                    if let Ok(req) = serde_json::from_str::<SocketRequest>(trimmed) {
                        if req.ping {
                            let resp = SocketResponse {
                                id: Some(req.id),
                                result: None,
                                error: None,
                                pong: Some(true),
                            };
                            let _ = writeln!(writer, "{}", serde_json::to_string(&resp).unwrap_or_default());
                            let _ = writer.flush();
                        } else {
                            let (tx, rx) = channel();
                            let req_id = if req.id.is_empty() { "default".to_string() } else { req.id };
                            let _ = proxy_inner.send_event(UserEvent::Fetch {
                                id: req_id,
                                url: req.url,
                                responder: tx,
                            });
                            match rx.recv_timeout(Duration::from_secs(25)) {
                                Ok(resp) => {
                                    let _ = writeln!(writer, "{}", serde_json::to_string(&resp).unwrap_or_default());
                                    let _ = writer.flush();
                                }
                                Err(_) => {
                                    let resp = SocketResponse {
                                        id: None,
                                        result: None,
                                        error: Some("TIMEOUT".to_string()),
                                        pong: None,
                                    };
                                    let _ = writeln!(writer, "{}", serde_json::to_string(&resp).unwrap_or_default());
                                    let _ = writer.flush();
                                }
                            }
                        }
                    }
                    line.clear();
                }
            });
        }
    });

    let mut pending_responders: HashMap<String, Sender<SocketResponse>> = HashMap::new();
    let mut last_active = Instant::now();
    let idle_timeout = Duration::from_secs(180);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(500));

        if last_active.elapsed() >= idle_timeout {
            eprintln!("[solver] inactivo por 3 minutos. Cerrando solver para liberar memoria (~100MB)...");
            let sock_path = get_solver_socket_path();
            let _ = fs::remove_file(&sock_path);
            *control_flow = ControlFlow::Exit;
            return;
        }

        match event {
            Event::UserEvent(UserEvent::Fetch { id, url, responder }) => {
                last_active = Instant::now();
                let fetch_path = if let Some(stripped) = url.strip_prefix(base_url) {
                    if stripped.is_empty() { "/" } else { stripped }
                } else {
                    &url
                };

                pending_responders.insert(id.clone(), responder);
                let escaped_path = fetch_path.replace('\\', "\\\\").replace('\'', "\\'");
                let escaped_id = id.replace('\\', "\\\\").replace('\'', "\\'");
                let eval_str = format!(
                    r#"
                    (async function() {{
                        try {{
                            let resp = await fetch('{}');
                            let jsonVal = null;
                            try {{ jsonVal = await resp.json(); }} catch(e) {{ }}
                            window.ipc.postMessage(JSON.stringify({{
                                id: '{}',
                                status: resp.status,
                                result: jsonVal,
                                error: ''
                            }}));
                        }} catch (err) {{
                            window.ipc.postMessage(JSON.stringify({{
                                id: '{}',
                                status: 500,
                                result: null,
                                error: err.toString()
                            }}));
                        }}
                    }})();
                    "#,
                    escaped_path, escaped_id, escaped_id
                );
                let _ = webview.evaluate_script(&eval_str);
            }
            Event::UserEvent(UserEvent::IpcResult(msg)) => {
                if let Ok(ipc) = serde_json::from_str::<IpcMessage>(&msg) {
                    if let Some(responder) = pending_responders.remove(&ipc.id) {
                        let res_str = ipc.result.map(|v| v.to_string());
                        let has_res = res_str.is_some();
                        if ipc.status != 403 && ipc.status != 503 && has_res {
                            let _ = responder.send(SocketResponse {
                                id: Some(ipc.id),
                                result: res_str,
                                error: None,
                                pong: None,
                            });
                        } else if !ipc.error.is_empty() {
                            let _ = responder.send(SocketResponse {
                                id: Some(ipc.id),
                                result: None,
                                error: Some(ipc.error),
                                pong: None,
                            });
                        } else {
                            let _ = responder.send(SocketResponse {
                                id: Some(ipc.id),
                                result: res_str,
                                error: None,
                                pong: None,
                            });
                        }
                    }
                }
            }
            _ => (),
        }
    });
}
