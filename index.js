function copyStringToMemory(mem, buf, str) {
  var u16 = new Uint16Array(mem.buffer, buf);
  for (let i = 0; i < str.length; i++) {
    u16[i] = str.charCodeAt(i);
  }
}

/**
 * Search for approximate matches for `pattern` in `text` allowing up to
 * `maxErrors` errors.
 *
 * @param {Object} wasm - Exports object of the compiled WebAssembly implementation
 * @param {string} text - Text to search in
 * @param {string} pattern - String to search for in `text`
 * @param {number} maxErrors - Maximum number of errors to allow
 * @return {Match[]} Array of matches
 */
function search(wasm, text, pattern, maxErrors) {
  // nb. Allocate a buffer that is one item longer than the text to avoid an
  // issue with trying to allocate zero-length buffers.
  const textPtr = wasm.alloc_char_buffer(text.length + 1);
  const patPtr = wasm.alloc_char_buffer(pattern.length + 1);

  copyStringToMemory(wasm.memory, textPtr, text);
  copyStringToMemory(wasm.memory, patPtr, pattern);

  const matchCount = wasm.search(
    textPtr,
    text.length,
    patPtr,
    pattern.length,
    maxErrors
  );

  wasm.free_char_buffer(textPtr);
  wasm.free_char_buffer(patPtr);

  const matches = [];
  for (let m = 0; m < matchCount; m++) {
    const start = wasm.match_start(m);
    const end = wasm.match_end(m);
    const errors = wasm.match_errors(m);
    matches.push({ start, end, errors });
  }
  wasm.clear_matches();

  return matches;
}

module.exports = { search };
