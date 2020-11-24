const fs = require("fs");

const { search } = require('../index');

const wasmCode = fs.readFileSync("./dist/search.wasm");
const wasmMod = new WebAssembly.Module(wasmCode);
const wasmInstance = new WebAssembly.Instance(wasmMod);

const matches = search(wasmInstance.exports, "hello world", "wrld", 5);
console.log("matches", matches);
