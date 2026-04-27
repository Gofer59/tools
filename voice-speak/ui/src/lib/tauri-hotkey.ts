export type HotkeyEvent = {
  tool: string;
  state: 'pressed' | 'released';
  captured?: string;
};

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function testHotkey(): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('test_hotkey');
}

export async function listenHotkeyTriggered(
  cb: (e: HotkeyEvent) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen<HotkeyEvent>('hotkey-triggered', (e) => cb(e.payload));
}

export async function listenHotkeyTestArmed(cb: () => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen('hotkey-test-armed', () => cb());
}
