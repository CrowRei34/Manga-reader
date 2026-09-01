use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ReadingActivity {
    pub title: String,
    pub chapter: String,
    pub cover_url: Option<String>,
    pub is_adult: bool,
    pub show_adult: bool,
}

enum Command {
    Set(ReadingActivity),
    Clear,
    Stop,
}

/// Cliente no bloqueante: toda comunicación con Discord ocurre en un hilo
/// dedicado y se reintenta cada 15 segundos si Discord todavía no está abierto.
pub struct DiscordPresence {
    tx: Sender<Command>,
    status: Arc<AtomicU8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

impl DiscordPresence {
    pub fn start(client_id: &str) -> Option<Self> {
        let client_id = client_id.trim();
        if client_id.is_empty() || !client_id.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(AtomicU8::new(1));
        let worker_status = Arc::clone(&status);
        let client_id = client_id.to_owned();
        std::thread::Builder::new()
            .name("bakeneko-discord".into())
            .spawn(move || {
                let mut client = DiscordIpcClient::new(client_id);
                let mut connected = false;
                let mut current: Option<ReadingActivity> = None;
                loop {
                    let command = match rx.recv_timeout(Duration::from_secs(15)) {
                        Ok(command) => Some(command),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => break,
                    };
                    match command {
                        Some(Command::Set(activity)) => current = Some(activity),
                        Some(Command::Clear) => {
                            current = None;
                            if connected && client.clear_activity().is_err() {
                                connected = false;
                                worker_status.store(0, Ordering::Relaxed);
                            }
                            continue;
                        }
                        Some(Command::Stop) => break,
                        None => {}
                    }

                    let Some(reading) = current.as_ref() else { continue };
                    if !connected {
                        connected = client.connect().is_ok();
                        worker_status.store(if connected { 2 } else { 0 }, Ordering::Relaxed);
                    }
                    if connected && set_reading_activity(&mut client, reading).is_err() {
                        connected = false;
                        worker_status.store(0, Ordering::Relaxed);
                    }
                }
                if connected {
                    let _ = client.clear_activity();
                    let _ = client.close();
                }
            })
            .ok()?;
        Some(Self { tx, status })
    }

    pub fn set_reading(&self, activity: ReadingActivity) {
        let _ = self.tx.send(Command::Set(activity));
    }

    pub fn clear(&self) {
        let _ = self.tx.send(Command::Clear);
    }

    pub fn status(&self) -> ConnectionStatus {
        match self.status.load(Ordering::Relaxed) {
            2 => ConnectionStatus::Connected,
            1 => ConnectionStatus::Connecting,
            _ => ConnectionStatus::Disconnected,
        }
    }
}

impl Drop for DiscordPresence {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Stop);
    }
}

fn set_reading_activity(
    client: &mut DiscordIpcClient,
    reading: &ReadingActivity,
) -> Result<(), discord_rich_presence::error::Error> {
    let private = reading.is_adult && !reading.show_adult;
    let title = discord_text(if private { "Leyendo una obra" } else { &reading.title });
    let chapter = discord_text(if private { "Contenido privado" } else { &reading.chapter });
    let mut assets = activity::Assets::new().large_text(&title);
    if !private {
        if let Some(cover) = reading.cover_url.as_deref() {
            assets = assets.large_image(cover);
        }
    }
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    client.set_activity(
        activity::Activity::new()
            .activity_type(activity::ActivityType::Watching)
            .status_display_type(activity::StatusDisplayType::Details)
            .details(&title)
            .state(&chapter)
            .assets(assets)
            .timestamps(activity::Timestamps::new().start(started)),
    )
}

fn discord_text(value: &str) -> String {
    let mut text: String = value.trim().chars().take(128).collect();
    if text.is_empty() {
        text.push_str("Bakeneko");
    } else if text.chars().count() == 1 {
        text.push('\u{3164}');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_text_is_never_too_short_or_too_long() {
        assert_eq!(discord_text(""), "Bakeneko");
        assert_eq!(discord_text("A").chars().count(), 2);
        assert_eq!(discord_text(&"x".repeat(200)).chars().count(), 128);
    }
}
