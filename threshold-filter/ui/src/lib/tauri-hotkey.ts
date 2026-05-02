export type HotkeyCapturedEvent = {
  captured: string;
  which: string;
};

export type HotkeyTestArmedEvent = {
  which: string;
};

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function testHotkey(which: string): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('test_hotkey', { which });
}

export async function listenHotkeyCaptured(
  cb: (e: HotkeyCapturedEvent) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen<HotkeyCapturedEvent>('hotkey-captured', (e) => cb(e.payload));
}

export async function listenHotkeyTestArmed(
  cb: (e: HotkeyTestArmedEvent) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen<HotkeyTestArmedEvent>('hotkey-test-armed', (e) => cb(e.payload));
}
