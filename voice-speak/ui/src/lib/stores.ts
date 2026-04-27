import { writable } from 'svelte/store';

export type Config = {
  hotkey: string;
  voice: string;
  speed: number;
  noise_scale: number;
  noise_w_scale: number;
  python_bin: string;
};

export const DEFAULT_CONFIG: Config = {
  hotkey: 'Ctrl+Alt+V',
  voice: 'en_US-lessac-medium',
  speed: 1.0,
  noise_scale: 0.667,
  noise_w_scale: 0.8,
  python_bin: 'python3',
};

export const config = writable<Config>(DEFAULT_CONFIG);

export type DownloadProgress = {
  bytes: number;
  total: number | null;
  speed_bps: number;
};

export const downloadProgress = writable<Record<string, DownloadProgress>>({});
