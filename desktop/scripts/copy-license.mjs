import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const source = resolve(__dirname, "../../LICENSE");
const destDir = resolve(__dirname, "../public/legal");
const dest = resolve(destDir, "LICENSE");

mkdirSync(destDir, { recursive: true });
copyFileSync(source, dest);
console.log(`Copied ${source} -> ${dest}`);
