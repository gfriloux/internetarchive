{pkgs, ...}:
pkgs.runCommand "rustfmt-check" {
  nativeBuildInputs = [pkgs.rustfmt];
} ''
  cp -r ${../../src} ./src
  rustfmt --check --config tab_spaces=2 ./src/lib.rs ./src/metadata.rs ./src/download.rs
  touch $out
''
