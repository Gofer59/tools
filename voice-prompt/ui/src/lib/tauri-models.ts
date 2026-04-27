export type ModelEntry = {
  id: string;
  kind: 'Whisper' | 'Piper';
  display_name: string;
  language: string;
  size_bytes: number;
  license: string;
  urls: string[];
  sha256: string | null;
  multilingual: boolean;
};

export type LocalModel = {
  id: string;
  kind: 'Whisper' | 'Piper';
  display_name: string;
  language: string;
  size_bytes: number;
  source: 'Catalog' | 'User';
  paths: string[];
};

export type DownloadProgressEvent = {
  id: string;
  bytes: number;
  total: number | null;
  speed_bps: number;
};

export type DownloadCompleteEvent = {
  id: string;
  sha256: string;
  path: string;
};

export type DownloadErrorEvent = {
  id: string;
  message: string;
};

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function listCatalogModels(): Promise<ModelEntry[]> {
  if (!isTauri()) return [];
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ModelEntry[]>('list_catalog_models');
}

export async function listLocalModels(): Promise<LocalModel[]> {
  if (!isTauri()) return [];
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<LocalModel[]>('list_local_models');
}

export async function downloadModel(id: string): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('download_model', { id });
}

export async function cancelDownload(id: string): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('cancel_download', { id });
}

export async function addCustomModel(path: string): Promise<LocalModel | null> {
  if (!isTauri()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<LocalModel>('add_custom_model', { path });
}

export async function deleteLocalModel(id: string): Promise<void> {
  if (!isTauri()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('delete_local_model', { id });
}

export async function listenDownloadProgress(
  cb: (e: DownloadProgressEvent) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen<DownloadProgressEvent>('download-progress', (e) => cb(e.payload));
}

export async function listenDownloadComplete(
  cb: (e: DownloadCompleteEvent) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen<DownloadCompleteEvent>('download-complete', (e) => cb(e.payload));
}

export async function listenDownloadError(
  cb: (e: DownloadErrorEvent) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen<DownloadErrorEvent>('download-error', (e) => cb(e.payload));
}
