import { writable } from 'svelte/store';

export type Config = {
  region_select_hotkey: string;
  toggle_on_top_hotkey: string;
  default_threshold: number;
  default_invert: boolean;
  default_always_on_top: boolean;
  auto_start_overlay: boolean;
};

export const DEFAULT_CONFIG: Config = {
  region_select_hotkey: 'F10',
  toggle_on_top_hotkey: 'F9',
  default_threshold: 128,
  default_invert: false,
  default_always_on_top: true,
  auto_start_overlay: true,
};

export const config = writable<Config>(DEFAULT_CONFIG);
