build:
  cargo build

test:
  cargo test -- --nocapture
  pre-commit run --all-files
