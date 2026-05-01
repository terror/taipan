set dotenv-load

default:
	just --list

alias f := fmt
alias r := run
alias t := test

all: build test clippy fmt-check

[group: 'bench']
bench *args:
  cargo bench --bench benchmarks {{ args }}

[group: 'misc']
build:
  cargo build

[group: 'web']
build-wasm:
  wasm-pack build crates/taipan-wasm --target web --out-dir www/src/wasm

[group: 'check']
check:
 cargo check

[group: 'check']
ci: test clippy forbid
  cargo fmt --all -- --check
  cargo update --locked --package taipan

[group: 'check']
clippy:
  cargo clippy --all --all-targets

[group: 'format']
fmt:
  cargo fmt

[group: 'format']
fmt-check:
  cargo fmt --all -- --check

[group: 'check']
forbid:
  ./bin/forbid

[group: 'misc']
install:
  cargo install -f taipan

[group: 'dev']
install-dev-deps:
  cargo install cargo-watch

[group: 'release']
publish:
  ./bin/publish

[group: 'dev']
run *args:
  cargo run {{ args }}

[group: 'test']
test:
  cargo test

[group: 'test']
test-release-workflow:
  -git tag -d test-release
  -git push origin :test-release
  git tag test-release
  git push origin test-release

[group: 'release']
update-changelog:
  echo >> CHANGELOG.md
  git log --pretty='format:- %s' >> CHANGELOG.md

[group: 'dev']
watch +COMMAND='test':
  cargo watch --clear --exec "{{COMMAND}}"

[group: 'web']
[working-directory: 'www']
web-build: build-wasm
  bun run build

[group: 'web']
[working-directory: 'www']
web-dev: build-wasm
  bun run dev

[group: 'web']
[working-directory: 'www']
web-format:
  bun run format

[group: 'web']
[working-directory: 'www']
web-install:
  bun install
