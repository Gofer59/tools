import { writable } from 'svelte/store';

export type Config = {
  hotkey_quick_capture: string;
  hotkey_select_region: string;
  hotkey_stop_tts: string;
  ocr_language: string;
  delivery_mode: string;
  tts_voice: string;
  tts_speed: number;
};

export const DEFAULT_CONFIG: Config = {
  hotkey_quick_capture: 'F9',
  hotkey_select_region: 'F10',
  hotkey_stop_tts: 'F11',
  ocr_language: 'eng',
  delivery_mode: 'clipboard',
  tts_voice: 'en_US-lessac-medium',
  tts_speed: 1.0,
};

export const config = writable<Config>(DEFAULT_CONFIG);
