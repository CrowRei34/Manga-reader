Bakeneko portátil para Linux x86_64

El paquete incluye un runtime Java reducido; no incluye un JDK de desarrollo.

Requisitos del sistema:
  - Linux x86_64 con glibc 2.35 o posterior.
  - Una sesión gráfica X11 o Wayland con sus bibliotecas habituales.
  - Vulkan/OpenGL funcionales para la aceleración de la interfaz.

1. Descomprime el archivo completo.
2. Entra en la carpeta Bakeneko-Portable-vVERSION-Linux-x86_64.
3. Ejecuta ./bakeneko

No requiere Java instalado y no usa FUSE ni monta una AppImage. No muevas
únicamente el lanzador: la carpeta app debe permanecer junto a él.

La release se distribuye para x86_64. Para arquitecturas distintas hace falta
compilar un binario y un runtime Java específicos.

Para capturar un diagnóstico:
  ./bakeneko 2>&1 | tee bakeneko.log
