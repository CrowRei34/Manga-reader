{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc cargo rustfmt clippy
    jdk21            # para compilar el daemon JVM
    pkg-config
    gtk3             # backend de Iced (si se usa tiny-skia no hace falta)
  ];
}
