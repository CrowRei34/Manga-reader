use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::{Duration, Instant};

use base64::Engine;
use gtk::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
    platform::unix::WindowExtUnix,
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
    #[serde(default)]
    is_image: bool,
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

fn get_cache_dir() -> PathBuf {
    let xdg_cache = env::var("XDG_CACHE_HOME").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.cache", home)
    });
    PathBuf::from(xdg_cache).join("bakeneko")
}

fn cache_stem(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

fn get_solver_socket_path() -> PathBuf {
    let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        let uid = unsafe { libc::getuid() };
        format!("/tmp/bakeneko-{}", uid)
    });
    PathBuf::from(runtime_dir).join("bakeneko").join("solver.sock")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Matar solver inmediatamente cuando el proceso padre (Bakeneko / Java) muere
    unsafe {
        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
    }
    thread::spawn(|| {
        let initial_ppid = unsafe { libc::getppid() };
        loop {
            thread::sleep(Duration::from_secs(1));
            let current_ppid = unsafe { libc::getppid() };
            if current_ppid == 1 || current_ppid != initial_ppid {
                let sock_path = get_solver_socket_path();
                let _ = fs::remove_file(&sock_path);
                std::process::exit(0);
            }
        }
    });

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

    let cache_dir = get_cache_dir();
    let _ = fs::create_dir_all(&cache_dir);

    let base_url = "https://mangadot.net";

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // 2. Ventana invisible realizada: realize() inicializa el widget GTK
    // sin mapear ninguna ventana al servidor X11 / i3wm (0% posibilidad de ventana en escritorio)
    let window = WindowBuilder::new()
        .with_title("bakeneko-solver")
        .with_visible(false)
        .build(&event_loop)?;

    let gtk_win = window.gtk_window();
    gtk_win.set_skip_taskbar_hint(true);
    gtk_win.set_skip_pager_hint(true);
    gtk_win.set_type_hint(gdk::WindowTypeHint::Utility);
    gtk_win.set_decorated(false);
    gtk_win.realize();

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

    // Servidor Unix Domain Socket en un hilo dedicado
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
            eprintln!("[solver] Inactivo por 3 minutos. Cerrando solver para liberar memoria (~100MB)...");
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
                let is_image_req = fetch_path.contains("/chapters/")
                    || fetch_path.ends_with(".webp")
                    || fetch_path.ends_with(".jpg")
                    || fetch_path.ends_with(".jpeg")
                    || fetch_path.ends_with(".png");

                let eval_str = if is_image_req {
                    format!(
                        r#"
                        (async function() {{
                            try {{
                                let resp = await fetch('{}');
                                let blob = await resp.blob();
                                let fr = new FileReader();
                                fr.onloadend = () => {{
                                    let b64 = fr.result.split(',')[1];
                                    window.ipc.postMessage(JSON.stringify({{
                                        id: '{}',
                                        status: resp.status,
                                        result: {{ url: '{}', data: b64, is_image: true }},
                                        error: ''
                                    }}));
                                }};
                                fr.readAsDataURL(blob);
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
                        escaped_path, escaped_id, escaped_path, escaped_id
                    )
                } else {
                    format!(
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

                                if (jsonVal && jsonVal.images && Array.isArray(jsonVal.images)) {{
                                    for (let img of jsonVal.images) {{
                                        let imgPath = img.url || img.path;
                                        if (imgPath) {{
                                            let fullImgUrl = imgPath.startsWith('/') ? 'https://mangadot.net' + imgPath : imgPath;
                                            fetch(fullImgUrl)
                                                .then(r => r.blob())
                                                .then(b => {{
                                                    let fr = new FileReader();
                                                    fr.onloadend = () => {{
                                                        let b64 = fr.result.split(',')[1];
                                                        if (b64) {{
                                                            window.ipc.postMessage(JSON.stringify({{
                                                                id: '__cache_img__',
                                                                status: 200,
                                                                result: {{ url: fullImgUrl, data: b64, is_image: true }},
                                                                error: ''
                                                            }}));
                                                        }}
                                                    }};
                                                    fr.readAsDataURL(b);
                                                }})
                                                .catch(() => {{}});
                                        }}
                                    }}
                                }}
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
                    )
                };
                let _ = webview.evaluate_script(&eval_str);
            }
            Event::UserEvent(UserEvent::IpcResult(msg)) => {
                if let Ok(ipc) = serde_json::from_str::<IpcMessage>(&msg) {
                    if let Some(res) = &ipc.result {
                        if res.get("is_image").and_then(|v| v.as_bool()).unwrap_or(false) {
                            if let (Some(url_val), Some(data_val)) = (res.get("url"), res.get("data")) {
                                if let (Some(img_url), Some(b64_str)) = (url_val.as_str(), data_val.as_str()) {
                                    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64_str) {
                                        let stem = cache_stem(img_url);
                                        let full_url_stem = if img_url.starts_with('/') {
                                            cache_stem(&format!("https://mangadot.net{}", img_url))
                                        } else {
                                            stem.clone()
                                        };
                                        let ext = if img_url.contains(".webp") {
                                            "webp"
                                        } else if img_url.contains(".png") {
                                            "png"
                                        } else {
                                            "jpg"
                                        };
                                        let _ = fs::write(cache_dir.join(format!("{stem}.{ext}")), &bytes);
                                        let _ = fs::write(cache_dir.join(format!("{full_url_stem}.{ext}")), &bytes);
                                    }
                                }
                            }
                            if ipc.id == "__cache_img__" {
                                return;
                            }
                        }
                    }

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
