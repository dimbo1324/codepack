// Opening a project, from wherever the path came from.
//
// Two entry points reach this — the folder picker on the Project page and a folder
// dropped onto the window — and they must behave identically, including how a failure is
// reported. Keeping the sequence here rather than in the page is what guarantees that.
import { openProject, stopWatch } from "$lib/api/client";
import { reportError } from "$lib/stores/toasts.svelte";
import { resetForNewProject, wizard } from "$lib/stores/wizard.svelte";

class ProjectAction {
  opening = $state(false);
}

export const projectAction = new ProjectAction();

export async function openProjectAt(path: string): Promise<boolean> {
  projectAction.opening = true;
  try {
    const context = await openProject(path);
    // A watcher belongs to the project that started it. Left running, it would keep
    // reporting changes in the previous folder while the status bar named the new one.
    if (wizard.watchActive) {
      await stopWatch().catch(() => undefined);
      wizard.watchActive = false;
      wizard.watchChangedPaths = [];
    }
    resetForNewProject(context);
    return true;
  } catch (error) {
    reportError("project.openFailed", error);
    return false;
  } finally {
    projectAction.opening = false;
  }
}
