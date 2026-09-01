# Discord Rich Presence para Bakeneko

## Experiencia propuesta

Mientras el lector esté abierto, Discord mostraría:

- **Leyendo:** nombre del manga.
- **Detalle:** capítulo actual y modo de lectura.
- **Imagen grande:** portada del manga.
- **Tiempo:** tiempo transcurrido leyendo el capítulo.

Al salir del lector se cambiaría a “Explorando la biblioteca”, sin revelar la
última obra. La función estaría desactivada por defecto y tendría controles de
privacidad para ocultar título, portada o toda la actividad +18.

## Diseño técnico

1. Integrar Discord IPC únicamente en el proceso Rust; el daemon Java no debe
   conocer credenciales ni estado social.
2. Crear un servicio `DiscordPresence` que reciba eventos `ReaderOpened`,
   `ChapterChanged` y `ReaderClosed`.
3. Usar una imagen genérica de Bakeneko como respaldo. Discord Rich Presence no
   acepta cualquier archivo local como asset: para portadas dinámicas se necesita
   una URL pública compatible o un pequeño proxy/CDN de imágenes.
4. Nunca reenviar cabeceras privadas, cookies ni URLs firmadas de las fuentes.
   El proxy debe descargar, redimensionar y cachear únicamente portadas públicas.
5. Para contenido +18, aplicar por defecto “Leyendo una obra” con imagen genérica;
   el usuario podrá autorizar explícitamente mostrar el título.

## Entrega por fases

- **Fase 1:** presencia básica, nombre/capítulo, imagen estática y ajustes de privacidad.
- **Fase 2:** servicio opcional de portadas dinámicas con caché y borrado automático.
- **Fase 3:** botones opcionales para abrir la obra, solo cuando exista una URL pública segura.

La integración debe fallar silenciosamente si Discord no está instalado o el IPC
no está disponible; nunca debe retrasar la apertura del lector.
