# MangaReader (Bakeneko)

Un lector de manga desktop de alto rendimiento construido con **Rust** y la interfaz gráfica **Iced**, respaldado por un daemon IPC en Kotlin.

## Características

- 🚀 **UI Nativa en Rust (Iced):** Interfaz fluida y ligera.
- 📖 **Lector Webtoon y Paginado:** Soporte para vista vertical continua y paginada tradicional.
- ⚡ **Daemon IPC:** Arquitectura desacoplada mediante Unix Domain Sockets (JSON-RPC).
- 💾 **Gestión Local:** Base de datos SQLite local y caché de imágenes eficiente.

## Requisitos

- **Rust:** 1.75+
- **Java / JRE:** 21+ para desarrollo. El paquete portátil ya incluye Java.

## Ejecución

```bash
cargo run
```

## Paquete portátil con CPack

El release publica `Bakeneko-Portable-vX.Y.Z-Linux-x86_64.tar.gz`. No necesita
FUSE, AppImage ni una instalación de Java:

```bash
tar -xzf Bakeneko-Portable-vX.Y.Z-Linux-x86_64.tar.gz
cd Bakeneko-Portable-vX.Y.Z-Linux-x86_64
./bakeneko
```

Para generarlo localmente con un JDK 21:

```bash
JRE_HOME=/ruta/al/jdk-21 ./package_cpack.sh 0.2.6
```

## Instalación rápida

Para instalar o actualizar la última versión desde Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/CrowRei34/Manga-reader/main/install.sh | bash
```

El instalador descarga el paquete portátil desde GitHub, valida su SHA-256,
crea el acceso `~/.local/bin/bakeneko` y registra Bakeneko en el menú de
aplicaciones. No requiere `sudo` ni Java instalado.

## Actualizaciones del núcleo de parsers

GitHub Actions comprueba diariamente la rama principal de Futon Parsers. Cuando
encuentra una revisión nueva, actualiza la dependencia, reconstruye el daemon y
ejecuta las pruebas de Kotlin y Rust. Solo si todo termina correctamente abre un
pull request automático; nunca modifica directamente la rama principal.

La comprobación también se puede iniciar manualmente desde **Actions → Actualizar
núcleo Futon → Run workflow**.

## Licencia

[BSD 3-Clause](LICENSE)
