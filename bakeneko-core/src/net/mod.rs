use crate::error::NetError;
use crate::xdg::Xdg;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

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
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            root: Xdg::cache_root(),
            client: reqwest::Client::builder().user_agent("bakeneko-rs/0.1").build().unwrap(),
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
        let stem = Self::stem(url);
        // Cache hit con cualquier extensión conocida.
        for ext in ["jpg", "png", "webp", "gif"] {
            let p = self.root.join(format!("{stem}.{ext}"));
            if p.exists() {
                return p;
            }
        }
        // Todavía no existe: hint de la URL o .jpg.
        let ext = Self::url_ext(url).unwrap_or("jpg");
        self.root.join(format!("{stem}.{ext}"))
    }

    pub async fn get(&self, url: &str, headers: &HashMap<String, String>) -> Result<PathBuf, NetError> {
        let path = self.cached_path(url);
        if path.exists() {
            return Ok(path);
        }
        let mut req = self.client.get(url);
        for (k, v) in headers {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(NetError::Http { status: resp.status(), url: url.to_string() });
        }
        let bytes = resp.bytes().await?;
        std::fs::create_dir_all(&self.root)?;
        // Guarda con la extensión real (sniff > URL > jpg) para que iced
        // decodifique por extensión.
        let ext = Self::sniff_ext(&bytes)
            .or_else(|| Self::url_ext(url))
            .unwrap_or("jpg");
        let final_path = self.root.join(format!("{}.{ext}", Self::stem(url)));
        std::fs::write(&final_path, bytes)?;
        Ok(final_path)
    }

    /// Descarga (o reusa de caché) la imagen de `url` y devuelve un
    /// [`iced::widget::image::Handle`] listo para pintar, o `None` si la
    /// descarga o la lectura del archivo fallan.
    /// (Helper de render; los features usan `get` + paths por ahora.)
    #[allow(dead_code)]
    pub async fn get_handle(&self, url: &str, headers: &HashMap<String, String>) -> Option<iced::widget::image::Handle> {
        let path = self.get(url, headers).await.ok()?;
        let bytes = std::fs::read(path).ok()?;
        Some(iced::widget::image::Handle::from_bytes(bytes))
    }
}
