// DownloadManager: cola de descargas persistida en SQLite + eventos broadcast.
//
// Espejo del Dart `download_manager.dart`:
//  - `enqueue` inserta/actualiza el estado del capítulo a `Queued`.
//  - `poll_once` toma hasta `concurrency` trabajos `Queued`, los marca
//    `Downloading`, itera las páginas llamando al daemon y a la caché de
//    imágenes, actualiza el progreso y finalmente los marca `Done`.
//  - En error por trabajo: `Error` + evento `Errored`, y se continúa con
//    el siguiente lote (un fallo no aborta toda la tanda).
//
// `poll_once` es síncrono porque vive en el task-loop de `app` (Task 18),
// típicamente dentro de `spawn_blocking` o un hilo dedicado. Para ejecutar
// los métodos `async` del daemon (que son futures) creamos un `Runtime`
// propio del manager y usamos `block_on` por llamada: así `poll_once`
// funciona desde cualquier hilo que no esté ya dentro de un runtime
// (caso de los tests `#[test]` planos), y el integrador decide desde
// dónde invocar el worker.
//
// El módulo está completo y ejercitado por `tests/download_manager_test`,
// pero el binario sólo usa `new` + `subscribe` (cola sin botón "Descargar"
// en el UI todavía): de ahí el allow de dead_code.
#![allow(dead_code)]

use crate::daemon::api::MangaSourceApi;
use crate::db::dao::{download_dao, manga_dao};
use crate::error::{DbError, DownloadError};
use crate::models::{Chapter, DownloadState, Manga};
use crate::net::ImageCache;
use crate::xdg::Xdg;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;


#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Queued(i64, String),
    Progress {
        manga_id: i64,
        chapter_url: String,
        done: i32,
        total: i32,
    },
    Done(i64, String),
    Errored(i64, String, String),
}

fn block_on_compat<F: std::future::Future>(f: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(f)),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build fallback tokio runtime")
            .block_on(f),
    }
}


pub struct Inner {
    pub db: Arc<Mutex<Connection>>,
    pub daemon: Arc<dyn MangaSourceApi>,
    pub cache: Arc<ImageCache>,
    pub root: PathBuf,
    pub concurrency: usize,
}

#[derive(Clone)]
pub struct DownloadManager {
    inner: Arc<Inner>,
    tx: broadcast::Sender<DownloadEvent>,
}

impl DownloadManager {
    pub fn new(
        db: Arc<Mutex<Connection>>,
        daemon: Arc<dyn MangaSourceApi>,
        cache: Arc<ImageCache>,
        concurrency: usize,
    ) -> Self {
        let (tx, _) = broadcast::channel(64);
        let root = Xdg::downloads_root();
        Self {
            inner: Arc::new(Inner {
                db,
                daemon,
                cache,
                root,
                concurrency,
            }),
            tx,
        }
    }

    /// Encola la descarga del capítulo: asegura la fila de manga y upsert del
    /// estado `Queued`. Emite `DownloadEvent::Queued`.
    pub fn enqueue(&self, m: &Manga, ch: &Chapter) -> Result<(), DbError> {
        let conn = self.inner.db.lock().unwrap();
        let id = manga_dao::upsert(&conn, m, 0)?;
        download_dao::upsert(&conn, id, &ch.url, DownloadState::Queued)?;
        let _ = self.tx.send(DownloadEvent::Queued(id, ch.url.clone()));
        Ok(())
    }

    /// Suscripción a los eventos de descarga (cola de 64, broadcast).
    pub fn subscribe(&self) -> broadcast::Receiver<DownloadEvent> {
        self.tx.subscribe()
    }

    /// Procesa un lote (hasta `concurrency`) de trabajos `Queued`. Un fallo
    /// por trabajo marca ese capítulo `Error` y emite `Errored`, pero no
    /// aborta el resto de la tanda.
    pub fn poll_once(&self) -> Result<(), DownloadError> {
        let inner = self.inner.clone();
        let batch: Vec<_> = {
            let conn = inner.db.lock().unwrap();
            let jobs = download_dao::list_by_state(&conn, DownloadState::Queued)?;
            jobs.into_iter().take(inner.concurrency).collect()
        };
        for job in batch {
            if let Err(e) = self.run_job(&inner, job.manga_id, &job.chapter_url) {
                let conn = inner.db.lock().unwrap();
                let _ = download_dao::set_state(
                    &conn,
                    job.manga_id,
                    &job.chapter_url,
                    DownloadState::Error,
                );
                let _ = self.tx.send(DownloadEvent::Errored(
                    job.manga_id,
                    job.chapter_url.clone(),
                    e.to_string(),
                ));
            }
        }
        Ok(())
    }

    fn run_job(
        &self,
        inner: &Inner,
        manga_id: i64,
        chapter_url: &str,
    ) -> Result<(), DownloadError> {
        let source = {
            let conn = inner.db.lock().unwrap();
            download_dao::set_state(&conn, manga_id, chapter_url, DownloadState::Downloading)?;
            let manga = manga_dao::get_by_id(&conn, manga_id)?
                .ok_or_else(|| DbError::Sql(rusqlite::Error::QueryReturnedNoRows))?;
            manga.source
        };

        // Reconstruimos un `Chapter` mínimo desde (manga source, chapter_url):
        // el DAO de descargas sólo guarda (manga_id, chapter_url), y el DAO
        // de capítulos no restaura `source` de la fila. Evitamos depender de
        // que el capítulo exista en `chapter`: basta (`source`, `url`) para
        // hablar con el daemon; el resto de campos no se usan al descargar.
        let ch = Chapter {
            source: source.clone(),
            url: chapter_url.to_string(),
            title: String::new(),
            number: 0.0,
            volume: 0,
            scanlator: None,
            upload_date: 0,
            branch: None,
            blob: Default::default(),
            read: false,
        };

        let pages = block_on_compat(inner.daemon.chapter_pages(&source, &ch))?;
        let headers: HashMap<String, String> = block_on_compat(inner.daemon.source_headers(&source))
            .unwrap_or_default();
        let total = pages.len() as i32;
        let mut done = 0;
        for p in &pages {
            let url = block_on_compat(inner.daemon.page_url(&source, p))?;
            let _ = block_on_compat(inner.cache.get(&url, &headers))?;
            done += 1;
            {
                let conn = inner.db.lock().unwrap();
                download_dao::update_progress(&conn, manga_id, chapter_url, done, total)?;
            }
            let _ = self.tx.send(DownloadEvent::Progress {
                manga_id,
                chapter_url: chapter_url.to_string(),
                done,
                total,
            });
        }
        {
            let conn = inner.db.lock().unwrap();
            download_dao::set_state(&conn, manga_id, chapter_url, DownloadState::Done)?;
        }
        let _ = self.tx.send(DownloadEvent::Done(manga_id, chapter_url.to_string()));
        Ok(())
    }
}