// SPDX-License-Identifier: Apache-2.0
import type { DiagnosticsStatus } from "../../api";

/** What to say about the previous session, or nothing: a recorded panic wins over a plain unclean exit. */
export function crashNotice(s: DiagnosticsStatus | null): "crash" | "unclean" | null {
  if (!s) return null;
  if (s.previousCrash !== null) return "crash";
  return s.uncleanPreviousExit ? "unclean" : null;
}
