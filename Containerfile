FROM debian:13-slim

ARG BAKENEKO_VERSION=0.0.0

LABEL org.opencontainers.image.title="Bakeneko" \
      org.opencontainers.image.description="Lector de manga Bakeneko para Containerpak" \
      org.opencontainers.image.version="${BAKENEKO_VERSION}" \
      org.opencontainers.image.source="https://github.com/CrowRei34/Manga-reader"

# Bakeneko usa Iced/wgpu para la interfaz. Las bibliotecas se instalan dentro
# de la imagen para que el paquete no dependa de una distro concreta del host.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
      ca-certificates \
      libdbus-1-3 \
      libfontconfig1 \
      libfreetype6 \
      libgl1 \
      libglib2.0-0 \
      libvulkan1 \
      libwayland-client0 \
      libx11-6 \
      libx11-xcb1 \
      libxcomposite1 \
      libxcursor1 \
      libxdamage1 \
      libxext6 \
      libxfixes3 \
      libxi6 \
      libxkbcommon0 \
      libxrandr2 \
      libxrender1 \
    && rm -rf /var/lib/apt/lists/*

# El contenido de esta raíz lo prepara scripts/prepare-cpak-root.sh a partir
# del tarball CPack validado por scripts/validate-portable-package.sh.
COPY dist/cpak-root/ /

ENTRYPOINT ["/usr/bin/bakeneko"]
