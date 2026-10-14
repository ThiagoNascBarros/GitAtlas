// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { decodeWindow, toArrayBuffer } from "./api";

// Header-only window: magic AKRW, version 1, start 5, count 0, lanes 3.
const bytes = [65, 75, 82, 87, 1, 0, 5, 0, 0, 0, 0, 0, 0, 0, 3, 0];

describe("toArrayBuffer", () => {
  it("accepts every payload shape Tauri may return", () => {
    const fromArray = toArrayBuffer(bytes);
    const fromView = toArrayBuffer(new Uint8Array([0, 0, ...bytes]).subarray(2));
    const fromBuffer = toArrayBuffer(new Uint8Array(bytes).buffer);
    for (const b of [fromArray, fromView, fromBuffer]) {
      expect(decodeWindow(b)).toEqual({ start: 5, laneCount: 3, rows: [] });
    }
  });
  it("rejects anything else with a clear error", () => {
    expect(() => toArrayBuffer("nope")).toThrow(/unexpected graph window payload/);
  });
});
