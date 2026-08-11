//! Utilidades compartidas (sin dependencias pesadas).
//!
//! `now_millis` se usa para timestamps `i64` en DAOs (history, downloads...)
//! y refleja `DateTime.now().millisecondsSinceEpoch` del Dart original.

use std::time::{SystemTime, UNIX_EPOCH};

/// Milisegundos desde UNIX epoch (1970-01-01 UTC). Si el reloj del sistema
/// estuviera antes de epoch (caso patológico), devuelve 0.
pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}