use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::{WebContext, WebViewBuilder};

#[derive(Serialize, Deserialize)]
struct SolverResult {
    status: u16,
    #[serde(default)]
    cookie: String,
}

enum UserEvent {
    Solved(String),
}

fn get_data_dir() -> PathBuf {
    let xdg_data = env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.local/share", home)
    });
    PathBuf::from(xdg_data).join("bakeneko")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env::set_var("NO_AT_BRIDGE", "1");
    env::set_var("GTK_A11Y", "none");
    env::set_var("GST_DEBUG", "0");
    env::set_var("PULSE_SERVER", "");
    env::set_var("PIPEWIRE_REMOTE", "");

    let args: Vec<String> = env::args().collect();
    let target_url = if args.len() > 1 {
        args[1].clone()
    } else {
        "https://mangadot.net".to_string()
    };

    println!("[INFO] Abriendo mini-navegador de verificación para: {}", target_url);

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Bakeneko - Verificación de Cloudflare")
        .with_visible(true)
        .with_inner_size(tao::dpi::LogicalSize::new(850.0, 650.0))
        .build(&event_loop)?;

    let script = r#"
        let solved = false;
        let inFlight = false;
        setInterval(async () => {
            if (solved || inFlight) return;
            inFlight = true;
            try {
                let resp = await fetch('/api/manga?page=1');
                if (resp.status === 200) {
                    solved = true;
                    window.ipc.postMessage(JSON.stringify({
                        status: 200,
                        cookie: document.cookie || ""
                    }));
                }
            } catch (e) {
                // Sigue en reto o error de red
            } finally {
                inFlight = false;
            }
        }, 500);
    "#;

    let bakeneko_dir = get_data_dir();
    let profile_dir = bakeneko_dir.join("solver_profile");
    let _ = fs::create_dir_all(&profile_dir);
    let mut web_context = WebContext::new(Some(profile_dir.clone()));

    let _webview = WebViewBuilder::with_web_context(&mut web_context)
        .with_url(&target_url)
        .with_initialization_script(script)
        .with_ipc_handler(move |req| {
            let _ = proxy.send_event(UserEvent::Solved(req.body().clone()));
        })
        .build(&window)?;

    let start_time = Instant::now();
    let max_timeout = Duration::from_secs(120);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(200));

        if start_time.elapsed() > max_timeout {
            println!("[WARN] Tiempo de verificación agotado (120s).");
            *control_flow = ControlFlow::Exit;
        }

        match event {
            Event::UserEvent(UserEvent::Solved(msg)) => {
                if let Ok(res) = serde_json::from_str::<SolverResult>(&msg) {
                    if res.status == 200 {
                        println!("[OK] Verificación completada con éxito. Guardando sesión...");
                        
                        // Guardar cookie en archivo si se obtuvo
                        let cookie_path = bakeneko_dir.join("mangadot_cookies.txt");
                        let _ = fs::write(&cookie_path, &res.cookie);
                        
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("[INFO] Ventana de verificación cerrada por el usuario.");
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }
    });
}
