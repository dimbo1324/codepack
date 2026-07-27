// Chrome state that belongs to the window rather than to the export: whether the user
// collapsed the sidebar, and whether the log follows its own tail.
//
// Deliberately not persisted. These are per-sitting preferences, and writing them into
// the settings file would mean a stray click on a splitter becomes a durable change to a
// file the user also edits by hand.
class UiState {
  sidebarCollapsed = $state(false);
  logFollow = $state(true);
}

export const ui = new UiState();

export function toggleSidebar(): void {
  ui.sidebarCollapsed = !ui.sidebarCollapsed;
}
