use crate::error::NetError;
use crate::xdg::Xdg;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

/// Cache de imágenes en disco (`$XDG_CACHE_HOME/bakeneko/<sha256>.<ext>`).
/// La extensión es REAL (sniffing de magic bytes, fallback a la ext de la
/// URL): iced deduce el formato del `Handle::from_path` por extensión, así
/// que un `.img` genérico no decodifica nunca — bug raíz del lector vacío.
///
/// `get` descarga la primera vez y sirve el path cacheado después; `get_handle`
/// devuelve un `Handle` de iced listo para dibujar.

pub struct ImageCache {
    root: PathBuf,
    client: reqwest::Client,
    /// Evita descargar la misma portada varias veces cuando las fuentes
    /// federadas responden casi simultáneamente.
    inflight: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    /// Limita decodificación/red para no saturar la UI y el atlas de GPU.
    download_slots: Semaphore,
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            root: Xdg::cache_root(),
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .unwrap(),
            inflight: Mutex::new(HashMap::new()),
            download_slots: Semaphore::new(6),
        }
    }

    fn stem(url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Extensión desde la URL (sin query), si parece imagen.
    fn url_ext(url: &str) -> Option<&'static str> {
        let path = url.split(['?', '#']).next()?;
        let ext = path.rsplit('.').next()?;
        match ext.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => Some("jpg"),
            "png" => Some("png"),
            "webp" => Some("webp"),
            "gif" => Some("gif"),
            _ => None,
        }
    }

    /// Sniffing de magic bytes para extensión real.
    fn sniff_ext(bytes: &[u8]) -> Option<&'static str> {
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) { return Some("jpg"); }
        if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) { return Some("png"); }
        if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" { return Some("webp"); }
        if bytes.starts_with(b"GIF8") { return Some("gif"); }
        None
    }

    pub fn cached_path(&self, url: &str) -> PathBuf {
        let clean_url = if url.starts_with("//") {
            format!("https:{url}")
        } else {
            url.to_string()
        };
        let stem = Self::stem(&clean_url);
        let ext = Self::url_ext(&clean_url).unwrap_or("jpg");
        let direct_path = self.root.join(format!("{stem}.{ext}"));
        if direct_path.exists() {
            return direct_path;
        }
        // Fallback si fue guardado con otra extensión tras sniffing
        for known_ext in ["jpg", "png", "webp", "gif"] {
            if known_ext != ext {
                let p = self.root.join(format!("{stem}.{known_ext}"));
                if p.exists() {
                    return p;
                }
            }
        }
        direct_path
    }

    pub async fn get(&self, url: &str, headers: &HashMap<String, String>) -> Result<PathBuf, NetError> {
        let normalized_url = if url.starts_with("//") {
            format!("https:{url}")
        } else {
            url.to_string()
        };
        let path = self.cached_path(&normalized_url);
        if path.exists() {
            return Ok(path);
        }
        let url_lock = {
            let mut inflight = self.inflight.lock().await;
            inflight.entry(normalized_url.clone()).or_insert_with(|| Arc::new(Mutex::new(()))).clone()
        };
        let _url_guard = url_lock.lock().await;
        let path = self.cached_path(&normalized_url);
        if path.exists() {
            return Ok(path);
        }
        let _slot = self.download_slots.acquire().await.expect("semaphore de portadas cerrado");
        let mut req = self.client.get(&normalized_url)
            .header("Accept", "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8");
        for (k, v) in headers {
            req = req.header(k, v);
        }
        // Fallback para Referer si no viene especificado en headers
        if !headers.keys().any(|k| k.eq_ignore_ascii_case("referer")) {
            if normalized_url.contains("comick") {
                req = req.header("Referer", "https://comick.live/");
            } else if normalized_url.contains("mangadex") {
                req = req.header("Referer", "https://mangadex.org/");
            } else if let Ok(parsed) = reqwest::Url::parse(&normalized_url) {
                if let Some(host) = parsed.host_str() {
                    req = req.header("Referer", format!("https://{host}/"));
                }
            }
        }
        let mut resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(NetError::Http { status: resp.status(), url: normalized_url });
        }
        std::fs::create_dir_all(&self.root)?;
        let stem = Self::stem(&normalized_url);
        let tmp_path = self.root.join(format!("{stem}.downloading"));
        let mut file = tokio::fs::File::create(&tmp_path).await?;
        
        let mut sniff_buffer = Vec::with_capacity(32);
        let mut ext = Self::url_ext(&normalized_url);
        
        use tokio::io::AsyncWriteExt;
        while let Some(chunk) = resp.chunk().await? {
            if sniff_buffer.len() < 32 {
                let needed = 32 - sniff_buffer.len();
                let take = needed.min(chunk.len());
                sniff_buffer.extend_from_slice(&chunk[..take]);
                if ext.is_none() {
                    ext = Self::sniff_ext(&sniff_buffer);
                }
            }
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        drop(file);

        let final_ext = ext.or_else(|| Self::sniff_ext(&sniff_buffer)).unwrap_or("jpg");
        let final_path = self.root.join(format!("{stem}.{final_ext}"));
        tokio::fs::rename(&tmp_path, &final_path).await?;
        Ok(final_path)
    }

    /// Descarga (o reusa de caché) la imagen de `url` y devuelve un
    /// [`iced::widget::image::Handle`] listo para pintar, o `None` si la
    /// descarga o la lectura del archivo fallan.
    #[allow(dead_code)]
    pub async fn get_handle(&self, url: &str, headers: &HashMap<String, String>) -> Option<iced::widget::image::Handle> {
        let path = self.get(url, headers).await.ok()?;
        let bytes = std::fs::read(path).ok()?;
        Some(iced::widget::image::Handle::from_bytes(bytes))
    }
}
