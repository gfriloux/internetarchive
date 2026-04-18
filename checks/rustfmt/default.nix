{pkgs, ...}:
pkgs.runCommand "rustfmt-check" {
  nativeBuildInputs = [pkgs.rustfmt];
} ''
  cp -r ${../../src} ./src
  rustfmt --check ./src/lib.rs ./src/metadata.rs
  touch $out
''
