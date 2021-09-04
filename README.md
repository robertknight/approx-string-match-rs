# approx-string-match-rs

A Rust + WebAssembly library for approximate string matching.

This project is an experimental port of
[approx-string-match](https://github.com/robertknight/approx-string-match-js)
from JavaScript to Rust.

approx-string-match relies heavily on bitwise operations. WebAssembly supports
bitwise operations on larger integers (64bit) than JavaScript (32bit). As a
result a WebAssembly implementation should theoretically be twice as fast for
given (text length, pattern length, maximum error count) arguments.

In practice however I found that the overhead of marshalling data from
JavaScript to WebAssembly memory reduced the gains significantly. With some API
changes it might be possible to mitigate this cost. For example in scenarios
where the same text is searched many times for a pattern, the text only needs
to be marshalled once from JS to WASM.

## Prerequisites

You will need a modern Rust toolchain with the wasm32-unknown-unknown build
target installed and a recent version of Node. If you are using
[rustup](https://www.rust-lang.org/tools/install) to manage Rust, you can
install the WebAssembly target with:

```sh
rustup target add wasm32-unknown-unknown
```

## Usage

```sh
# Compile library. This creates a debug build of `dist/search.wasm`.
# Use `make build-release` to create a release build.
make build

# Run demo
node demo/demo.js
```
