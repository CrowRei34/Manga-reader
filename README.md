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

## Instalación y actualización

Para instalar Bakeneko por primera vez o actualizarlo a la última versión
publicada, cierra la aplicación y ejecuta desde Linux:

```bash
wget -qO- https://raw.githubusercontent.com/CrowRei34/Manga-reader/main/install.sh | bash
```

El mismo comando es seguro para actualizar: descarga el release más reciente,
valida su SHA-256 y cambia el enlace `current` a la nueva versión. También
puedes indicar la intención explícitamente:

```bash
wget -qO- https://raw.githubusercontent.com/CrowRei34/Manga-reader/main/install.sh | bash -s -- --update
```

Las versiones se guardan en `~/.local/share/bakeneko/releases/`; la biblioteca,
el historial y la configuración permanecen intactos. El instalador actualiza
`~/.local/bin/bakeneko` y el acceso del menú de aplicaciones, no requiere
`sudo` ni Java instalado. Si todavía no hay un release publicado, espera a que
termine el workflow **Publicar paquete portátil** en GitHub Actions. Para
retirar solo el programa (conservando la biblioteca):

```bash
wget -qO- https://raw.githubusercontent.com/CrowRei34/Manga-reader/main/install.sh | bash -s -- --uninstall
```

El instalador prefiere `wget`; si no está disponible, usa `curl` como respaldo.

## Actualizaciones del núcleo de parsers

GitHub Actions comprueba diariamente la rama principal de Futon Parsers. Cuando
encuentra una revisión nueva, actualiza la dependencia, reconstruye el daemon y
ejecuta las pruebas de Kotlin y Rust. Solo si todo termina correctamente abre un
pull request automático; nunca modifica directamente la rama principal.

La comprobación también se puede iniciar manualmente desde **Actions → Actualizar
núcleo Futon → Run workflow**.

## Licencia

[BSD 3-Clause](LICENSE)
