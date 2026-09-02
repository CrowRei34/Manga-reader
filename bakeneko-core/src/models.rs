use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manga {
    pub source: String,
    pub url: String,
    pub title: String,
    #[serde(rename = "publicUrl")] pub public_url: Option<String>,
    #[serde(default)] pub rating: f32,
    #[serde(rename = "isNsfw", default)] pub is_nsfw: bool,
    #[serde(rename = "coverUrl")] pub cover_url: Option<String>,
    #[serde(rename = "largeCoverUrl")] pub large_cover_url: Option<String>,
    pub description: Option<String>,
    #[serde(default)] pub authors: Vec<String>,
    pub state: Option<String>,
    #[serde(default)] pub chapters: Vec<Chapter>,
    #[serde(skip)] pub blob: serde_json::Map<String, serde_json::Value>,
}

impl Default for Manga {
    fn default() -> Self {
        Self {
            source: String::new(),
            url: String::new(),
            title: String::new(),
            public_url: None,
            rating: 0.0,
            is_nsfw: false,
            cover_url: None,
            large_cover_url: None,
            description: None,
            authors: Vec::new(),
            state: None,
            chapters: Vec::new(),
            blob: serde_json::Map::new(),
        }
    }
}

impl Manga {
    /// Clave compuesta source|url (usada por tests y futuras features).
    #[allow(dead_code)]
    pub fn key(&self) -> String { format!("{}|{}", self.source, self.url) }
    pub fn blob_json(&self) -> String {
        if self.blob.is_empty() { serde_json::json!({}).to_string() } else { serde_json::Value::Object(self.blob.clone()).to_string() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub source: String,
    pub url: String,
    pub title: String,
    #[serde(default)] pub number: f32,
    #[serde(default)] pub volume: i32,
    #[serde(default)] pub language: Option<String>,
    pub scanlator: Option<String>,
    #[serde(rename = "uploadDate", default)] pub upload_date: i64,
    pub branch: Option<String>,
    #[serde(skip)] pub blob: serde_json::Map<String, serde_json::Value>,
    /// Estado leído; escrito por el DownloadManager (`run_job`) y el flujo
    /// de lectura; aún sin lectura por el UI.
    #[serde(skip)] #[allow(dead_code)] pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub source: String,
    pub url: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    #[serde(default)] pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingReply { pub version: String, pub java: String }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadState { Idle, Queued, Downloading, Done, Error }

#[derive(Debug, Clone)]
pub struct DownloadEntry {
    pub manga_id: i64,
    pub chapter_url: String,
    pub state: DownloadState,
    pub total_pages: i32,
    pub done_pages: i32,
}

/// Categoría de la biblioteca (superficie de la DB; UI pendiente).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Category {
    pub id: Option<i64>,
    pub name: String,
    pub color: String,
    pub auto_download: bool,
    pub created_at: i64,
}

/// Entrada de historial (superficie de la DB; UI pendiente).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HistoryEntry {
    pub manga: Manga,
    pub chapter_index: i32,
    pub page_index: i32,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct MangaRef {
    pub source: String,
    pub url: String,
    pub title: String,
    pub cover_url: Option<String>,
}
