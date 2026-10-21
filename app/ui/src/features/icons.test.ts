// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { Redo2, Undo2 } from "lucide-solid";
import { MENU_ICONS, MenuAction, SHARED_ICON_PAIRS } from "./icons";

const entries = Object.entries(MENU_ICONS) as [MenuAction, unknown][];
const pairKey = (a: string, b: string) => [a, b].sort().join("|");

describe("MENU_ICONS", () => {
  it("gives every menu action a component", () => {
    for (const [, icon] of entries) expect(typeof icon).toBe("function");
  });

  it("keeps the toolbar's undo and redo icons out of the menus", () => {
    const values = entries.map(([, icon]) => icon);
    expect(values).not.toContain(Undo2);
    expect(values).not.toContain(Redo2);
  });

  it("uses one icon per action, except the declared same-meaning pairs", () => {
    const allowed = new Set(SHARED_ICON_PAIRS.map(([a, b]) => pairKey(a, b)));
    const byIcon = new Map<unknown, MenuAction[]>();
    for (const [name, icon] of entries) byIcon.set(icon, [...(byIcon.get(icon) ?? []), name]);
    for (const names of byIcon.values()) {
      for (let i = 0; i < names.length; i++) {
        for (let j = i + 1; j < names.length; j++) {
          expect(allowed.has(pairKey(names[i], names[j])), `${names[i]} and ${names[j]} share an icon`).toBe(true);
        }
      }
    }
  });

  it("only declares shared pairs that really share an icon", () => {
    const icon = (name: MenuAction) => MENU_ICONS[name];
    for (const [a, b] of SHARED_ICON_PAIRS) expect(icon(a), `${a} / ${b}`).toBe(icon(b));
  });
});
