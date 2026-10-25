// SPDX-License-Identifier: Apache-2.0
import type { DiagnosticsStatus } from "../../api";

/** Which notice to show after a previous run ended badly; a recorded panic wins over the generic one. */
export function crashNotice(s: DiagnosticsStatus | null): "crash" | "unclean" | null {
  if (!s) return null;
  if (s.previousCrash) return "crash";
  return s.uncleanPreviousExit ? "unclean" : null;
}

/** One-line summary of a recorded panic (the full text stays under "Details"). */
export function crashSummary(details: string): string {
  const line = details.split("\n").map((l) => l.trim()).find(Boolean) ?? "";
  return line.length > 160 ? line.slice(0, 159) + "…" : line;
}
