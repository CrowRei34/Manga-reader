# Discord Rich Presence para Bakeneko

## Experiencia implementada

Mientras el lector esté abierto, Discord mostraría:

- **Leyendo:** nombre del manga.
- **Detalle:** capítulo actual y modo de lectura.
- **Imagen grande:** portada del manga.
- **Tiempo:** tiempo transcurrido leyendo el capítulo.

Al salir del lector se cambiaría a “Explorando la biblioteca”, sin revelar la
última obra. La función estaría desactivada por defecto y tendría controles de
privacidad para ocultar título, portada o toda la actividad +18.

## Configuración

1. Crear una aplicación en <https://discord.com/developers/applications>.
2. Copiar su **Application ID**.
3. En Bakeneko, abrir **Ajustes → Discord Rich Presence**, pegar el ID y
   activar “Mostrar lo que estoy leyendo”.

No se utiliza ningún secreto de Discord. El Application ID es un identificador
público y queda guardado en la configuración local de Bakeneko.

## Diseño técnico

1. Integrar Discord IPC únicamente en el proceso Rust; el daemon Java no debe
   conocer credenciales ni estado social.
2. Crear un servicio `DiscordPresence` que reciba eventos `ReaderOpened`,
   `ChapterChanged` y `ReaderClosed`.
3. Enviar la URL pública de la portada como imagen grande, siguiendo el enfoque
   de Pear Desktop. Si Discord no admite una portada concreta, mantiene el texto
   de la actividad sin bloquear el lector.
4. Nunca reenviar cabeceras privadas, cookies ni URLs firmadas de las fuentes.
   El proxy debe descargar, redimensionar y cachear únicamente portadas públicas.
5. Para contenido +18, aplicar por defecto “Leyendo una obra” con imagen genérica;
   el usuario podrá autorizar explícitamente mostrar el título.

## Comportamiento

- Actualiza título, capítulo, portada y tiempo al abrir o cambiar de capítulo.
- Reintenta la conexión cada 15 segundos si Discord arranca más tarde.
- Detecta sockets normales, Flatpak, Vesktop y Snap mediante la biblioteca IPC.
- Limpia la actividad al salir del lector o cerrar la aplicación.
- Oculta título y portada +18 salvo autorización explícita.

La integración debe fallar silenciosamente si Discord no está instalado o el IPC
no está disponible; nunca debe retrasar la apertura del lector.
