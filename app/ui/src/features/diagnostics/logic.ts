// SPDX-License-Identifier: Apache-2.0
import type { DiagnosticsStatus } from "../../api";

/** Which notice to show after a previous run ended badly; a recorded panic wins over the generic one. */
export function crashNotice(s: DiagnosticsStatus | null): "crash" | "unclean" | null {
  if (!s) return null;
  if (s.previousCrash) return "crash";
  return s.uncleanPreviousExit ? "unclean" : null;
}
