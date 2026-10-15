// SPDX-License-Identifier: Apache-2.0
import type { RefInfo, RefKind } from "../../api";

/** Refs of one kind whose name contains the filter text (case-insensitive). */
export const filterRefs = (refs: RefInfo[], kind: RefKind, q: string) =>
  refs.filter((r) => r.kind === kind && r.name.toLowerCase().includes(q.trim().toLowerCase()));
