/**
 * Wrapper around a string held in WebAssembly memory.
 */
class WasmString {
  /**
   * Construct a string in WebAssembly memory from a JavaScript string.
   *
   * @param {object} wasm - WebAssembly exports from `lib/wasm.rs`.
   */
  constructor(wasm, str) {
    this.wasm = wasm;
    this.buffer = wasm.char_buf_alloc(str.length);

    const data = wasm.char_buf_data(this.buffer);
    const u16 = new Uint16Array(wasm.memory.buffer, data);
    for (let i = 0; i < str.length; i++) {
      u16[i] = str.charCodeAt(i);
    }
  }

  free() {
    this.wasm.char_buf_free(this.buffer);
    this.buffer = 0;
  }
}

/**
 * Search for approximate matches for `pattern` in `text` allowing up to
 * `maxErrors` errors.
 *
 * @param {Object} wasm - Exports object of the compiled WebAssembly implementation
 * @param {WasmString} text - Text to search in
 * @param {WasmString} pattern - String to search for in `text`
 * @param {number} maxErrors - Maximum number of errors to allow
 * @return {Match[]} Array of matches
 */
function search(wasm, text, pattern, maxErrors) {
  const matchVec = wasm.match_vec_alloc();

  wasm.search(matchVec, text.buffer, pattern.buffer, maxErrors);

  const matchCount = wasm.match_vec_len(matchVec);
  const matches = [];
  for (let m = 0; m < matchCount; m++) {
    const match = wasm.match_vec_get(matchVec, m);
    const start = wasm.match_start(match);
    const end = wasm.match_end(match);
    const errors = wasm.match_errors(match);
    matches.push({ start, end, errors });
  }

  wasm.match_vec_free(matchVec);
  return matches;
}

module.exports = { WasmString, search };
