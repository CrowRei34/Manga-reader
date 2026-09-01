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

## Actualizaciones del núcleo de parsers

GitHub Actions comprueba diariamente la rama principal de Futon Parsers. Cuando
encuentra una revisión nueva, actualiza la dependencia, reconstruye el daemon y
ejecuta las pruebas de Kotlin y Rust. Solo si todo termina correctamente abre un
pull request automático; nunca modifica directamente la rama principal.

La comprobación también se puede iniciar manualmente desde **Actions → Actualizar
núcleo Futon → Run workflow**.

## Licencia

[BSD 3-Clause](LICENSE)
