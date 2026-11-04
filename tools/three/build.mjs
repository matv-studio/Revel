// Assemble three.js pour Revel (D-160) : `npm ci && node build.mjs`. Écrit, dans
// `crates/revel-render/vendor/three/` : le moteur (`revel-three.min.js`), les décodeurs Draco et
// Basis (fichiers d'origine de three.js) et les licences. À refaire en changeant de version.
import { build } from "esbuild";
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "../../crates/revel-render/vendor/three");
const three = join(here, "node_modules/three");
const version = JSON.parse(readFileSync(join(three, "package.json"), "utf8")).version;
mkdirSync(out, { recursive: true });

await build({
  entryPoints: [join(here, "entry.js")],
  bundle: true,
  minify: true,
  format: "iife",
  target: "es2020",
  legalComments: "none",
  // Adresses par défaut des décodeurs (`new URL(…, import.meta.url)`) : la page ; Revel donne
  // toujours le dossier des décodeurs (`setDecoderPath`, `setTranscoderPath`).
  define: { "import.meta.url": "location.href" },
  banner: { js: `/*! three.js ${version} | MIT License | (c) 2010-2026 three.js authors */` },
  outfile: join(out, "revel-three.min.js"),
});

const libs = join(three, "examples/jsm/libs");
for (const [from, to] of [
  ["draco/gltf/draco_wasm_wrapper.js", "draco_wasm_wrapper.js"],
  ["draco/gltf/draco_decoder.wasm", "draco_decoder.wasm"],
  ["basis/basis_transcoder.js", "basis_transcoder.js"],
  ["basis/basis_transcoder.wasm", "basis_transcoder.wasm"],
]) copyFileSync(join(libs, from), join(out, to));
copyFileSync(join(three, "LICENSE"), join(out, "LICENSE"));
writeFileSync(
  join(out, "VERSION"),
  `three ${version} (MIT, LICENSE)\n` +
    "draco_wasm_wrapper.js, draco_decoder.wasm : Draco, Google, Apache-2.0 " +
    "(https://github.com/google/draco/blob/master/LICENSE)\n" +
    "basis_transcoder.js, basis_transcoder.wasm : Basis Universal, Binomial, Apache-2.0 " +
    "(https://github.com/BinomialLLC/basis_universal/blob/master/LICENSE)\n",
);
console.log(`three ${version} -> ${out}`);
