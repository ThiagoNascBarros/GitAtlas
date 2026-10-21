// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { crashNotice } from "./logic";
import type { DiagnosticsStatus } from "../../api";

const base: DiagnosticsStatus = { logDir: "/state/atlasgit", uncleanPreviousExit: false, previousCrash: null };

describe("crashNotice", () => {
  it("says nothing before the status arrives or after a clean exit", () => {
    expect(crashNotice(null)).toBeNull();
    expect(crashNotice(base)).toBeNull();
  });
  it("reports an unclean exit without a recorded panic", () => {
    expect(crashNotice({ ...base, uncleanPreviousExit: true })).toBe("unclean");
  });
  it("prefers the recorded panic over the generic unclean message", () => {
    const crash = { details: "boom\nlocal: a.rs:1" };
    expect(crashNotice({ ...base, uncleanPreviousExit: true, previousCrash: crash })).toBe("crash");
    expect(crashNotice({ ...base, previousCrash: crash })).toBe("crash");
  });
});
