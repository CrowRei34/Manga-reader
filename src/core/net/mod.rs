use crate::core::error::NetError;
use crate::core::xdg::Xdg;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

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

    pub fn cached_path(&self, url: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hex: String = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect();
        self.root.join(format!("{hex}.img"))
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
        std::fs::write(&path, bytes)?;
        Ok(path)
    }
}
