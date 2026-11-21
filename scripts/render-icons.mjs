// Renders the MEDORE icon SVGs to PNG and multi-size ICO files, and an
// optional preview sheet.  Usage:
//   node scripts/render-icons.mjs            -> src-tauri/icons/*
//   node scripts/render-icons.mjs --preview <out.png>
import { Resvg } from "@resvg/resvg-js";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const design = join(root, "design", "medore");
const svg = (name) => readFileSync(join(design, name), "utf8");

function png(svgText, size) {
  return new Resvg(svgText, { fitTo: { mode: "width", value: size }, background: "rgba(0,0,0,0)" }).render().asPng();
}

/** ICO with PNG-compressed entries (supported by Windows Vista and later). */
function ico(svgText, sizes) {
  const images = sizes.map((s) => ({ s, data: png(svgText, s) }));
  const header = Buffer.alloc(6 + 16 * images.length);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);
  let offset = header.length;
  images.forEach((img, i) => {
    const e = 6 + i * 16;
    header.writeUInt8(img.s >= 256 ? 0 : img.s, e);
    header.writeUInt8(img.s >= 256 ? 0 : img.s, e + 1);
    header.writeUInt8(0, e + 2);
    header.writeUInt8(0, e + 3);
    header.writeUInt16LE(1, e + 4);
    header.writeUInt16LE(32, e + 6);
    header.writeUInt32LE(img.data.length, e + 8);
    header.writeUInt32LE(offset, e + 12);
    offset += img.data.length;
  });
  return Buffer.concat([header, ...images.map((i) => i.data)]);
}

const dark = svg("medore-icon-dark.svg");
const light = svg("medore-icon-light.svg");
const args = process.argv.slice(2);

if (args[0] === "--preview") {
  const uri = (t) => "data:image/svg+xml;base64," + Buffer.from(t).toString("base64");
  const row = (y, bg, sizes) => {
    let x = 40;
    let out = `<rect x="0" y="${y}" width="1400" height="120" fill="${bg}"/>`;
    for (const [t, s] of sizes) {
      out += `<image href="${uri(t)}" x="${x}" y="${y + 60 - s / 2}" width="${s}" height="${s}"/>`;
      x += s + 36;
    }
    return out;
  };
  const sheet = `<svg xmlns="http://www.w3.org/2000/svg" width="1400" height="860" viewBox="0 0 1400 860">
    <rect width="1400" height="860" fill="#7a7a7a"/>
    <rect x="0" y="0" width="700" height="600" fill="#202020"/>
    <rect x="700" y="0" width="700" height="600" fill="#f3f3f3"/>
    <image href="${uri(dark)}" x="110" y="60" width="480" height="480"/>
    <image href="${uri(light)}" x="810" y="60" width="480" height="480"/>
    ${row(620, "#1c1c1c", [[dark, 64], [dark, 48], [dark, 32], [dark, 24], [dark, 16]])}
    ${row(740, "#eeeeee", [[light, 64], [light, 48], [light, 32], [light, 24], [light, 16]])}
  </svg>`;
  writeFileSync(args[1], new Resvg(sheet, { fitTo: { mode: "original" } }).render().asPng());
  console.log("preview ->", args[1]);
} else {
  const out = join(root, "src-tauri", "icons");
  mkdirSync(out, { recursive: true });
  // The executable / installer icon: dark (obsidian) is the brand default.
  writeFileSync(join(out, "icon.ico"), ico(dark, [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]));
  writeFileSync(join(out, "medore-dark.ico"), ico(dark, [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]));
  writeFileSync(join(out, "medore-light.ico"), ico(light, [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]));
  for (const [name, s] of [["32x32.png", 32], ["64x64.png", 64], ["128x128.png", 128], ["128x128@2x.png", 256], ["icon.png", 512]]) {
    writeFileSync(join(out, name), png(dark, s));
  }
  writeFileSync(join(out, "window-dark.png"), png(dark, 256));
  writeFileSync(join(out, "window-light.png"), png(light, 256));
  // Brand marks for the interface
  const pub = join(root, "public", "brand");
  mkdirSync(pub, { recursive: true });
  writeFileSync(join(pub, "medore-icon-dark.svg"), dark);
  writeFileSync(join(pub, "medore-icon-light.svg"), light);
  console.log("icons ->", out);
}
