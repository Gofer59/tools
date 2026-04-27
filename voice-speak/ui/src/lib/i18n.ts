import { writable, get } from 'svelte/store';

type Lang = 'en' | 'fr';

export const strings = {
  en: {
    settings: 'Settings',
    models: 'Models',
    about: 'About',
    hotkey: 'Hotkey',
    voice: 'Voice',
    speed: 'Speed',
    noise: 'Noise scale',
    noise_w: 'Noise width',
    python: 'Python interpreter',
    press_hotkey: 'Press your hotkey…',
    test_voice: 'Test voice',
    play: 'Play',
    stop: 'Stop',
    discard: 'Discard',
    reset: 'Reset to defaults',
    export: 'Export',
    import: 'Import',
    add_custom: 'Add custom model',
    delete: 'Delete',
    cancel: 'Cancel',
    download: 'Download',
    downloaded: 'Downloaded',
    downloading: 'Downloading',
    available: 'Available',
    no_telemetry: 'No telemetry collected.',
    license: 'License',
    repo: 'Source code',
    version: 'Version',
    language: 'Language',
  },
  fr: {
    settings: 'Paramètres',
    models: 'Modèles',
    about: 'À propos',
    hotkey: 'Raccourci',
    voice: 'Voix',
    speed: 'Vitesse',
    noise: 'Échelle de bruit',
    noise_w: 'Largeur de bruit',
    python: 'Interpréteur Python',
    press_hotkey: 'Appuyez sur votre raccourci…',
    test_voice: 'Tester la voix',
    play: 'Lire',
    stop: 'Arrêter',
    discard: 'Annuler',
    reset: 'Réinitialiser',
    export: 'Exporter',
    import: 'Importer',
    add_custom: 'Ajouter un modèle',
    delete: 'Supprimer',
    cancel: 'Annuler',
    download: 'Télécharger',
    downloaded: 'Téléchargé',
    downloading: 'Téléchargement',
    available: 'Disponible',
    no_telemetry: 'Aucune télémétrie collectée.',
    license: 'Licence',
    repo: 'Code source',
    version: 'Version',
    language: 'Langue',
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
