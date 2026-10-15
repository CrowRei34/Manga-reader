Bakeneko portátil para Linux x86_64

1. Descomprime el archivo completo.
2. Entra en la carpeta Bakeneko-Portable-vVERSION-Linux-x86_64.
3. Ejecuta ./bakeneko

No requiere Java instalado y no usa FUSE ni monta una AppImage. No muevas
únicamente el lanzador: la carpeta app debe permanecer junto a él.

El solver para fuentes protegidas está incluido. En Linux utiliza WebKitGTK
4.1, que debe existir en el sistema:
  Void Linux:      sudo xbps-install -S libwebkit2gtk41
  Debian/Ubuntu:   sudo apt install libwebkit2gtk-4.1-0
  Arch/Manjaro:    sudo pacman -S webkit2gtk-4.1

Para capturar un diagnóstico:
  ./bakeneko 2>&1 | tee bakeneko.log
