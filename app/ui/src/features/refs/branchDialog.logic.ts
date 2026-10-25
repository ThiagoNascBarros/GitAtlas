// SPDX-License-Identifier: Apache-2.0
// Pure rules of the "New branch" dialog. The name and the Git Flow base come from the core
// (`branch_plan`); this file only decides which options are shown and when Create is allowed.

export type BranchKind = "free" | "feature" | "fix" | "release" | "hotfix";

/** Name template used until the Settings screen provides one. */
export const DEFAULT_BRANCH_TEMPLATE = "{ticket} {title}";

const FLOW_KINDS: readonly BranchKind[] = ["feature", "release", "hotfix"];

/** Git Flow types only appear when the repository has a `develop` branch. */
export function kindOptions(flowEnabled: boolean): BranchKind[] {
  return flowEnabled ? ["free", "feature", "fix", "release", "hotfix"] : ["free", "fix"];
}

export function isFlowKind(kind: BranchKind): boolean {
  return FLOW_KINDS.includes(kind);
}

/** Base used for the branch: Git Flow kinds take the core's base, the others the chosen one. */
export function effectiveBase(kind: BranchKind, coreBase: string | undefined, chosen: string): string | null {
  if (isFlowKind(kind)) return coreBase ?? null;
  return chosen;
}

/** Create is allowed only with a current, non-empty preview and a known base. */
export function canCreate(preview: { name: string } | null, previewFailed: boolean, base: string | null): boolean {
  return !!preview && preview.name.length > 0 && !previewFailed && base !== null;
}

/** Git Flow branches found in the repository: `develop` must exist to enable the flow. */
export function flowBranches(localNames: readonly string[]): { flowEnabled: boolean; main: string; develop: string } {
  const has = (n: string) => localNames.includes(n);
  return {
    flowEnabled: has("develop"),
    main: has("main") ? "main" : has("master") ? "master" : "main",
    develop: "develop",
  };
}
