// Preview-tree filtering.
//
// Done as one pass that rebuilds a pruned tree, rather than by asking each row "does any
// descendant match?" while rendering: the second shape is quadratic, and the preview of a
// real project is tens of thousands of nodes. A directory survives when any file under it
// survives, so filtering never orphans a match behind a collapsed parent.
import type { FileStatus, TreeNode } from "$lib/api/types";

export interface TreeFilter {
  query: string;
  status: FileStatus | "all";
}

export function isFilterActive(filter: TreeFilter): boolean {
  return filter.query.trim().length > 0 || filter.status !== "all";
}

export function filterTree(node: TreeNode, filter: TreeFilter): TreeNode | null {
  if (!isFilterActive(filter)) return node;
  const needle = filter.query.trim().toLowerCase();

  if (!node.is_dir) {
    const matchesQuery = !needle || node.path.toLowerCase().includes(needle);
    const matchesStatus = filter.status === "all" || node.status === filter.status;
    return matchesQuery && matchesStatus ? node : null;
  }

  const children = (node.children ?? [])
    .map((child) => filterTree(child, filter))
    .filter((child): child is TreeNode => child !== null);

  return children.length > 0 ? { ...node, children } : null;
}

/** Files only — directories are synthetic, and counting them would inflate "3 matches"
 * into "11 matches" for the same three files. */
export function countFiles(node: TreeNode): number {
  if (!node.is_dir) return 1;
  return (node.children ?? []).reduce((total, child) => total + countFiles(child), 0);
}
