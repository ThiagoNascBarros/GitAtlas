// SPDX-License-Identifier: Apache-2.0
// One icon per menu action (UI-02). Lucide is the only icon set in the app. Rules:
// - an icon never means two different actions (the audit found Redo2/Undo2 reused for
//   cherry-pick/revert, GitMerge for merge/rebase, RotateCcw for all three resets, Check for checkout);
// - the toolbar keeps Undo2/Redo2 for undo/redo, so no menu uses them;
// - the reset levels escalate: neutral, then a square variant, then a warning for hard reset.
// Actions with the same meaning in two menus may share an icon: list them in SHARED_ICON_PAIRS.
import { Cherry, Copy, CornerDownRight, CornerUpLeft, GitBranchPlus, GitCompareArrows, GitMerge, ListOrdered, LogIn, Pencil, RotateCcw, RotateCcwSquare, Tag, Trash2, TriangleAlert, Upload } from "lucide-solid";

export const MENU_ICONS = {
  // Commit menu (graph row)
  "commit.cherryPick": Cherry,
  "commit.revert": CornerUpLeft,
  "commit.resetSoft": RotateCcw,
  "commit.resetMixed": RotateCcwSquare,
  "commit.resetHard": TriangleAlert,
  "commit.newBranch": GitBranchPlus,
  "commit.tag": Tag,
  "commit.rebaseInteractive": ListOrdered,
  "commit.checkout": CornerDownRight,
  "commit.copySha": Copy,
  // Branch / tag menu (left panel)
  "ref.checkout": LogIn,
  "ref.checkoutDetached": CornerDownRight,
  "ref.merge": GitMerge,
  "ref.rebase": GitCompareArrows,
  "ref.rebaseInteractive": ListOrdered,
  "ref.pushTag": Upload,
  "ref.deleteTag": Trash2,
  "ref.newBranch": GitBranchPlus,
  "ref.rename": Pencil,
  "ref.delete": Trash2,
  "ref.forceDelete": TriangleAlert,
} as const;

export type MenuAction = keyof typeof MENU_ICONS;

/** Same meaning in two menus: allowed to share one icon. Anything else sharing an icon is a bug. */
export const SHARED_ICON_PAIRS: readonly (readonly [MenuAction, MenuAction])[] = [
  ["commit.checkout", "ref.checkoutDetached"],
  ["commit.newBranch", "ref.newBranch"],
  ["commit.rebaseInteractive", "ref.rebaseInteractive"],
  ["ref.delete", "ref.deleteTag"],
  // "Danger": both discard work (hard reset, force delete).
  ["commit.resetHard", "ref.forceDelete"],
];
