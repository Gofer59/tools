import { writable } from 'svelte/store';

export type Config = {
  push_to_talk_key: string;
  whisper_model: string;
  language: string;
  vad_filter: boolean;
  python_bin: string;
  compute_type: string;
};

export const DEFAULT_CONFIG: Config = {
  push_to_talk_key: 'Ctrl+Alt+Space',
  whisper_model: 'small',
  language: 'en',
  vad_filter: true,
  python_bin: 'python3',
  compute_type: 'int8',
};

export const config = writable<Config>(DEFAULT_CONFIG);

export type DownloadProgress = {
  bytes: number;
  total: number | null;
  speed_bps: number;
};

export const downloadProgress = writable<Record<string, DownloadProgress>>({});
