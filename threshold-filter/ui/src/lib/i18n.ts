import { writable, get } from 'svelte/store';

type Lang = 'en' | 'fr';

export const strings = {
  en: {
    settings: 'Settings',
    about: 'About',
    region_select_hotkey: 'Select Region Hotkey',
    toggle_on_top_hotkey: 'Toggle Always-on-Top Hotkey',
    default_threshold: 'Default Threshold',
    default_invert: 'Invert by default',
    default_always_on_top: 'Always on top by default',
    auto_start_overlay: 'Auto-start overlay on launch',
    overlay_status: 'Overlay',
    press_hotkey: 'Press your hotkey…',
    discard: 'Discard',
    reset: 'Reset to defaults',
    export: 'Export',
    import: 'Import',
    no_telemetry: 'No telemetry collected.',
    license: 'License',
    repo: 'Source code',
    version: 'Version',
  },
  fr: {
    settings: 'Paramètres',
    about: 'À propos',
    region_select_hotkey: 'Raccourci sélection région',
    toggle_on_top_hotkey: 'Raccourci toujours au premier plan',
    default_threshold: 'Seuil par défaut',
    default_invert: 'Inverser par défaut',
    default_always_on_top: 'Toujours au premier plan par défaut',
    auto_start_overlay: 'Démarrer l\'incrustation au lancement',
    overlay_status: 'Incrustation',
    press_hotkey: 'Appuyez sur votre raccourci…',
    discard: 'Annuler',
    reset: 'Réinitialiser',
    export: 'Exporter',
    import: 'Importer',
    no_telemetry: 'Aucune télémétrie collectée.',
    license: 'Licence',
    repo: 'Code source',
    version: 'Version',
  },
} as const;

type Key = keyof typeof strings.en;

function detect(): Lang {
  if (typeof navigator !== 'undefined' && navigator.language?.startsWith('fr')) {
    return 'fr';
  }
  return 'en';
}

export const locale = writable<Lang>(detect());

export function t(k: Key): string {
  return strings[get(locale)][k];
}
