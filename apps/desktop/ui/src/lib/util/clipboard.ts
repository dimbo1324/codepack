// Copy-to-clipboard, without the clipboard plugin.
//
// Tauri's `plugin-clipboard-manager` would need a permission granted in
// `capabilities/default.json`, and widening that file to let the UI write the system
// clipboard is a security decision, not a convenience one. The DOM API needs no
// permission at all and works inside the app's own webview, so the narrower tool is the
// right one here.
//
// The plugin was declared in `package.json` until 2026-07-27 and imported by nothing —
// it was removed rather than left as a dependency this file's own comment argued
// against using (finding 5, 2026-07-27 audit). Its Rust half was never added at all.
//
// The `execCommand` branch is the fallback for a webview that refuses the async API
// (older WebView2 builds treat the custom protocol as an insecure context).
export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return copyViaSelection(text);
  }
}

function copyViaSelection(text: string): boolean {
  const carrier = document.createElement("textarea");
  carrier.value = text;
  carrier.setAttribute("readonly", "");
  carrier.style.position = "fixed";
  carrier.style.opacity = "0";
  document.body.appendChild(carrier);
  try {
    carrier.select();
    return document.execCommand("copy");
  } catch {
    return false;
  } finally {
    carrier.remove();
  }
}
