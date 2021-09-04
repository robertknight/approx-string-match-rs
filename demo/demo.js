const fs = require("fs");

const { WasmString, search } = require("../index");

const wasmCode = fs.readFileSync("./dist/search.wasm");
const wasmMod = new WebAssembly.Module(wasmCode);
const wasmInstance = new WebAssembly.Instance(wasmMod);

const text =
  "Many years later, as he faced the firing squad, Colonel Aureliano Buendía was to remember that distant afternoon when his father took him to discover ice.";
const pattern = "faced t firing squad";

const wasmText = new WasmString(wasmInstance.exports, text);
const wasmPattern = new WasmString(wasmInstance.exports, pattern);

const start = Date.now();
const matches = search(
  wasmInstance.exports,
  wasmText,
  wasmPattern,
  pattern.length / 2
);
const end = Date.now();

for (let m of matches) {
  const actual = text.substr(m.start, m.end - m.start);
  console.log(
    `Found match "${actual}" for pattern "${pattern}" (${m.errors} errors)`
  );
}

console.log("Time (ms):", end - start);
