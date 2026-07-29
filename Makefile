# tono — a deterministic sound engine (library + CLI).
#
# `make help` lists every target. `make verify` is exactly what CI runs.

BIN     := target/release/tono
RELEASE_BRANCH ?= master

.DEFAULT_GOAL := help
.PHONY: help run build build-release install desktop play python wheel python-test python-smoke capi wasm test bench fmt lint check pre-commit-checks verify verify-native site version release hooks clean

help: ## List available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

run: build-release ## Build release and print the CLI usage
	$(BIN) --help

build: ## Debug build
	cargo build

build-release: ## Optimized release build → target/release/tono
	cargo build --release

install: ## Install the `tono` CLI into ~/.cargo/bin
	cargo install --path .

desktop: ## Build the native desktop studio (Tauri + cpal + MIDI) — off the default build; gated by 'make verify-native'
	cargo build -p tono-desktop --release
	@echo "→ run it:  target/release/tono-desktop"

EXAMPLE ?= playground

play: ## Run a tono-play example (EXAMPLE=<name>, see crates/tono-play/examples) — off the default build; gated by 'make verify-native'
	cargo run -p tono-play --example $(EXAMPLE)

python: ## Build the Python extension into the active venv (maturin develop) — off the default build; gated by 'make verify-native' + 'make python-test'
	maturin develop -m crates/tono-py/Cargo.toml

wheel: ## Build a release abi3 wheel for the Python bindings → target/wheels/
	maturin build --release -m crates/tono-py/Cargo.toml

python-test: ## Run the Python determinism + typed-API tests (build the extension first: make python)
	python3 crates/tono-py/tests/smoke.py
	python3 crates/tono-py/tests/test_typed.py

python-smoke: ## Build the extension as a wheel, install it, run the smoke + typed-API tests (what the Python workflow runs)
	python3 -m pip install --upgrade pip maturin numpy
	maturin build --out dist -m crates/tono-py/Cargo.toml
	python3 -m pip install --no-index --find-links dist --force-reinstall --no-deps tono
	python3 crates/tono-py/tests/smoke.py
	python3 crates/tono-py/tests/test_typed.py

capi: ## Build the C ABI (tono-capi) and run the C smoke test against its staticlib — off the default build
	cargo build -p tono-capi
	cargo run -q -p tono-capi --example emit_program > target/capi-smoke.program.json
	$(CC) -std=c11 -Wall -Wextra -Werror -I crates/tono-capi crates/tono-capi/tests/smoke.c target/debug/libtono_capi.a -o target/capi-smoke
	target/capi-smoke target/capi-smoke.program.json

WASM_CRATE := crates/tono-wasm
WASM_OUT   := target/wasm32-unknown-unknown/release/tono_wasm.wasm

wasm: ## Build the WASM face (tono-wasm) → crates/tono-wasm/pkg — off the default build; runs wasm-bindgen when the CLI is installed, else prints the exact command
	@rustup target list --installed | grep -qx wasm32-unknown-unknown || \
		{ echo "→ adding the wasm32-unknown-unknown target"; rustup target add wasm32-unknown-unknown; }
	cargo build -p tono-wasm --target wasm32-unknown-unknown --release
	@if command -v wasm-bindgen >/dev/null 2>&1; then \
		wasm-bindgen --target web --out-dir $(WASM_CRATE)/pkg $(WASM_OUT); \
		echo "→ bindings in $(WASM_CRATE)/pkg — try the player:"; \
		echo "    cd $(WASM_CRATE) && python3 -m http.server 8000  # → http://localhost:8000/js/example.html"; \
	else \
		V=$$(sed -n '/^name = "wasm-bindgen"/{n;s/^version = "\([^"]*\)"/\1/p;}' Cargo.lock | head -1); \
		echo "→ built $(WASM_OUT)"; \
		echo "  wasm-bindgen-cli not found — install the version the crate was built with and rerun:"; \
		echo "    cargo install wasm-bindgen-cli --version $$V"; \
		echo "    wasm-bindgen --target web --out-dir $(WASM_CRATE)/pkg $(WASM_OUT)"; \
	fi

test: ## Run the test suite
	cargo test

bench: ## Criterion benchmarks for the tono-core render hot path (report-only; NOT part of 'make verify')
	cargo bench -p tono-core

fmt: ## Format all sources
	cargo fmt --all

lint: ## Clippy with warnings denied
	cargo clippy --all-targets -- -D warnings

check: fmt lint test ## Pre-commit gate (mutating): format + clippy + tests

pre-commit-checks: ## CI lint gate (non-mutating): fmt --check + clippy. Pair with 'make test'.
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings

verify: pre-commit-checks test ## Exactly what CI runs (fmt --check + clippy + test) - non-mutating

site: ## Assemble the GitHub Pages site into _site/ (what the Pages workflow deploys)
	mkdir -p _site/audio _site/img
	cp site/index.html site/architecture.html _site/
	cp docs/examples/audio/*.mp4 _site/audio/
	cp docs/logo.png docs/logo-wordmark.png docs/river-flows-spectrogram.png _site/img/

version: ## Print the workspace version (the single version parser — release + CI both use it)
	@sed -n '/^\[workspace\.package\]/,/^\[/ s/^version = "\([^"]*\)".*/\1/p' Cargo.toml

verify-native: ## Lint + test the off-CI native crates (desktop/play/py); --all-targets compiles their examples too
	cargo clippy -p tono-desktop -p tono-play -p tono-py --all-targets -- -D warnings
	cargo test -p tono-desktop -p tono-play

release: ## Cut a release: guard clean master, tag vX.Y.Z from Cargo.toml, push (CI publishes to crates.io)
	@[ "$$(git branch --show-current)" = "$(RELEASE_BRANCH)" ] || { echo "Release only from $(RELEASE_BRANCH)."; exit 1; }
	@git diff --quiet && git diff --cached --quiet || { echo "Working tree dirty — commit before releasing."; exit 1; }
	@V=$$($(MAKE) -s version); \
		echo "→ Releasing v$$V"; \
		if git rev-parse "v$$V" >/dev/null 2>&1; then echo "tag v$$V exists — bump version in Cargo.toml first."; exit 1; fi; \
		git tag -a "v$$V" -m "v$$V" && git push origin "v$$V"; \
		echo "✓ Tagged v$$V. The release workflow publishes to crates.io — watch GitHub Actions."

hooks: ## Install the git hooks (pre-commit: lint gate; pre-push: refuse master + make verify)
	git config core.hooksPath .githooks
	@echo "git hooks enabled (.githooks)"

clean: ## Remove build artifacts
	cargo clean
