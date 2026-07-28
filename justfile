set dotenv-load

export RUST_LOG := 'info'

alias d := dev
alias f := fmt
alias t := test

default:
  just --list

[group: 'dev']
build:
  bun run build

[group: 'dev']
dev:
  bun run tauri dev

[group: 'format']
fmt:
  bun x prettier --write . --ignore-unknown
  cargo fmt --manifest-path src-tauri/Cargo.toml

[group: 'dev']
test:
  bun test
  cargo test --manifest-path src-tauri/Cargo.toml

[group: 'dev']
typeshare:
  typeshare --config-file typeshare.toml --lang typescript --output-file src/lib/types.ts src-tauri
