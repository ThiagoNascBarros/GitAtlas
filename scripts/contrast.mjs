// SPDX-License-Identifier: Apache-2.0
// Checks WCAG 2.2 AA contrast for every theme in themes/ (design-tokens §4).
// Usage: node scripts/contrast.mjs   → exits 1 on any failure.
import { readFileSync, readdirSync } from "node:fs";

const PAIRS = [
  ["text.primary", "bg.canvas", 4.5], ["text.primary", "bg.surface", 4.5], ["text.primary", "bg.selected", 4.5],
  ["text.secondary", "bg.surface", 4.5], ["text.secondary", "bg.raised", 4.5], ["text.muted", "bg.surface", 4.5],
  ["text.muted", "bg.canvas", 4.5], ["text.onAccent", "accent.default", 4.5], ["status.added", "bg.surface", 4.5],
  ["status.modified", "bg.surface", 4.5], ["status.deleted", "bg.surface", 4.5], ["status.renamed", "bg.surface", 4.5],
  ["status.conflict", "bg.surface", 4.5], ["diff.addText", "diff.addBg", 4.5], ["diff.delText", "diff.delBg", 4.5],
  ["text.primary", "diff.addBg", 4.5], ["text.primary", "diff.delBg", 4.5], ["accent.default", "bg.surface", 3],
  ["focus.ring", "bg.surface", 3], ["border.strong", "bg.surface", 3],
];

export function luminance(hex) {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
export function ratio(a, b) {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

const dir = new URL("../themes/", import.meta.url);
let failures = 0;
for (const file of readdirSync(dir).filter((f) => f.endsWith(".theme.json"))) {
  const theme = JSON.parse(readFileSync(new URL(file, dir), "utf8"));
  const t = (k) => theme.tokens[`color.${k}`];
  const hc = theme.kind.startsWith("high-contrast");
  for (const [fg, bg, min] of PAIRS) {
    const need = hc && min === 4.5 ? 7 : min;
    if (!t(fg) || !t(bg)) { console.error(`${theme.id}: missing ${fg} or ${bg}`); failures++; continue; }
    const r = ratio(t(fg), t(bg));
    if (r < need) { console.error(`${theme.id}: ${fg} on ${bg} = ${r.toFixed(2)} < ${need}`); failures++; }
  }
  for (let i = 0; i < 10; i++) {
    const r = ratio(t(`lane.${i}`), t("bg.canvas"));
    if (r < 3) { console.error(`${theme.id}: lane.${i} = ${r.toFixed(2)} < 3`); failures++; }
  }
  console.log(`${theme.id}: checked`);
}
console.log(failures ? `${failures} failure(s)` : "0 failures");
process.exit(failures ? 1 : 0);
