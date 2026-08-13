# MangaReader (Bakeneko)

Un lector de manga desktop de alto rendimiento construido con **Rust** y la interfaz gráfica **Iced**, respaldado por un daemon IPC en Kotlin.

## Características

- 🚀 **UI Nativa en Rust (Iced):** Interfaz fluida y ligera.
- 📖 **Lector Webtoon y Paginado:** Soporte para vista vertical continua y paginada tradicional.
- ⚡ **Daemon IPC:** Arquitectura desacoplada mediante Unix Domain Sockets (JSON-RPC).
- 💾 **Gestión Local:** Base de datos SQLite local y caché de imágenes eficiente.

## Requisitos

- **Rust:** 1.75+
- **Java / JRE:** 17+ (para el daemon IPC)

## Ejecución

```bash
cargo run
```

## Licencia

[BSD 3-Clause](LICENSE)
