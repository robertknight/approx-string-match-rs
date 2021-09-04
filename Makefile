.PHONY: build
build: dist
	cargo build --target wasm32-unknown-unknown
	cp target/wasm32-unknown-unknown/debug/approx_string_match_rs.wasm dist/search.wasm

.PHONY: test
test:
	cargo test

.PHONY: build-release
build-release: dist
	cargo build --target wasm32-unknown-unknown --release
	cp target/wasm32-unknown-unknown/release/approx_string_match_rs.wasm dist/search.wasm

dist:
	mkdir -p dist/

.PHONY: clean
clean:
	cargo clean
	rm -rf dist
