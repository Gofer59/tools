import type { Config } from './stores';
import { DEFAULT_CONFIG } from './stores';

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function getConfig(): Promise<Config> {
  if (!isTauri()) return DEFAULT_CONFIG;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<Config>('get_config');
}

export async function updateConfig(partial: Partial<Config>): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('update_config', { partial });
}

export async function listenConfigApplied(
  cb: (p: { field: string; value: unknown }) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<{ field: string; value: unknown }>(
    'config-applied',
    (e) => cb(e.payload),
  );
  return unlisten;
}

export async function listenTtsState(
  cb: (p: { state: string }) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<{ state: string }>('tts-state', (e) => cb(e.payload));
  return unlisten;
}
