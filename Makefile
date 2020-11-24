.PHONY: build
build:
	cargo build --target wasm32-unknown-unknown

.PHONY: build-release
build-release:
	cargo build --target wasm32-unknown-unknown --release
	wasm-snip --snip-rust-fmt-code --snip-rust-panicking-code target/wasm32-unknown-unknown/release/approx_string_match_rs.wasm -o /tmp/search.wasm
	mkdir -p dist
	wasm-opt -Oz --strip-debug /tmp/search.wasm -o dist/search.wasm
