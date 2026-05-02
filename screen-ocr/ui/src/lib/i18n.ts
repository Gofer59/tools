import { writable, get } from 'svelte/store';

type Lang = 'en' | 'fr';

export const strings = {
  en: {
    settings: 'Settings',
    about: 'About',
    hotkey_quick_capture: 'Quick Capture',
    hotkey_select_region: 'Select Region',
    hotkey_stop_tts: 'Stop TTS',
    ocr_language: 'OCR Language',
    delivery_mode: 'Delivery Mode',
    tts_voice: 'TTS Voice',
    tts_speed: 'TTS Speed',
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
    hotkey_quick_capture: 'Capture rapide',
    hotkey_select_region: 'Sélectionner région',
    hotkey_stop_tts: 'Arrêter TTS',
    ocr_language: 'Langue OCR',
    delivery_mode: 'Mode de livraison',
    tts_voice: 'Voix TTS',
    tts_speed: 'Vitesse TTS',
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
