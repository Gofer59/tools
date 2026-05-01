import { writable } from 'svelte/store';

export type Config = {
  push_to_talk_key: string;
  whisper_model: string;
  tiny_model: string;
  preview_mode: string;
  tiny_device: string;
  large_device: string;
  language: string;
  vad_filter: boolean;
  python_bin: string;
  compute_type: string;
  live_window_seconds: number;
  live_hop_ms: number;
};

export const DEFAULT_CONFIG: Config = {
  push_to_talk_key: 'Ctrl+Alt+Space',
  whisper_model: 'small',
  tiny_model: 'tiny',
  preview_mode: 'inline-replace',
  tiny_device: 'cpu',
  large_device: 'cpu',
  language: 'en',
  vad_filter: true,
  python_bin: 'python3',
  compute_type: 'int8',
  live_window_seconds: 6.0,
  live_hop_ms: 250,
};

export const config = writable<Config>(DEFAULT_CONFIG);

export type DownloadProgress = {
  bytes: number;
  total: number | null;
  speed_bps: number;
};

export const downloadProgress = writable<Record<string, DownloadProgress>>({});

export const partialTranscript = writable<string>('');
export const finalTranscript   = writable<string>('');
