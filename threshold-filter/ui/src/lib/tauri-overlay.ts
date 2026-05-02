export type OverlayStateEvent = {
  running: boolean;
};

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function startOverlay(): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('start_overlay');
}

export async function stopOverlay(): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('stop_overlay');
}

export async function isOverlayRunning(): Promise<boolean> {
  if (!isTauri()) return false;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<boolean>('is_overlay_running');
}

export async function listenOverlayState(
  cb: (e: OverlayStateEvent) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen<OverlayStateEvent>('overlay-state', (e) => cb(e.payload));
}
