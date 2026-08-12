{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc cargo rustfmt clippy
    jdk21            # para compilar el daemon JVM
    pkg-config
    gtk3             # backend de Iced
    wayland
    libxkbcommon
    libX11
    libXcursor
    libXrandr
    libXi
    vulkan-loader
    libGL
    libglvnd
    mesa.drivers
  ];

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    wayland
    libxkbcommon
    libX11
    libXcursor
    libXrandr
    libXi
    vulkan-loader
    libGL
    libglvnd
    mesa.drivers
  ]) + ":/run/opengl-driver/lib:/usr/lib/x86_64-linux-gnu";
}


