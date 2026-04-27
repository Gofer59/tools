<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { config, DEFAULT_CONFIG, type Config } from '$lib/stores';
  import { getConfig, updateConfig, listenConfigApplied } from '$lib/tauri-config';
  import { testHotkey, listenHotkeyTriggered, listenHotkeyTestArmed } from '$lib/tauri-hotkey';
  import { t } from '$lib/i18n';

  // ── state ──────────────────────────────────────────────────────────────────
  let local = $state<Config>({ ...$config });
  let saved = $state<Config>({ ...$config });
  let dirty = $derived(JSON.stringify(local) !== JSON.stringify(saved));
  let capturedHotkey = $state<string | null>(null);
  let armingHotkey = $state(false);
  let saveMsg = $state('');
  let debounceTimers: Record<string, ReturnType<typeof setTimeout>> = {};

  let unlistenConfig: (() => void) | null = null;
  let unlistenHotkey: (() => void) | null = null;
  let unlistenArmed: (() => void) | null = null;

  const WHISPER_MODELS = [
    { id: 'tiny',     label: 'Tiny (~75 MB)' },
    { id: 'base',     label: 'Base (~145 MB)' },
    { id: 'small',    label: 'Small (~466 MB)' },
    { id: 'medium',   label: 'Medium (~1.5 GB)' },
    { id: 'large-v3', label: 'Large v3 (~2.9 GB)' },
  ];

  const LANGUAGES = [
    { id: 'en',   label: 'English' },
    { id: 'fr',   label: 'French' },
    { id: 'de',   label: 'German' },
    { id: 'es',   label: 'Spanish' },
    { id: 'it',   label: 'Italian' },
    { id: 'pt',   label: 'Portuguese' },
    { id: 'nl',   label: 'Dutch' },
    { id: 'ru',   label: 'Russian' },
    { id: 'zh',   label: 'Chinese' },
    { id: 'ja',   label: 'Japanese' },
    { id: 'auto', label: 'Auto-detect' },
  ];

  const COMPUTE_TYPES = [
    { id: 'int8',          label: 'int8 — fastest' },
    { id: 'int8_float16',  label: 'int8_float16' },
    { id: 'float16',       label: 'float16' },
    { id: 'float32',       label: 'float32 — most accurate' },
  ];

  // ── lifecycle ──────────────────────────────────────────────────────────────
  onMount(async () => {
    const remote = await getConfig();
    local = { ...remote };
    saved = { ...remote };
    config.set(remote);

    unlistenConfig = await listenConfigApplied(({ field }) => {
      saveMsg = `✓ ${field} applied`;
      setTimeout(() => (saveMsg = ''), 2000);
    });

    unlistenHotkey = await listenHotkeyTriggered((e) => {
      if (armingHotkey && e.captured) {
        capturedHotkey = e.captured;
        armingHotkey = false;
      }
    });

    unlistenArmed = await listenHotkeyTestArmed(() => {
      armingHotkey = true;
      capturedHotkey = null;
    });
  });

  onDestroy(() => {
    unlistenConfig?.();
    unlistenHotkey?.();
    unlistenArmed?.();
    Object.values(debounceTimers).forEach(clearTimeout);
  });

  // ── helpers ────────────────────────────────────────────────────────────────
  function debounce<K extends keyof Config>(field: K, value: Config[K], delay = 250) {
    clearTimeout(debounceTimers[field as string]);
    debounceTimers[field as string] = setTimeout(async () => {
      try {
        await updateConfig({ [field]: value });
        saved = { ...local };
      } catch {
        local = { ...saved };
      }
    }, delay);
  }

  function set<K extends keyof Config>(field: K, value: Config[K]) {
    local = { ...local, [field]: value };
    debounce(field, value);
  }

  async function startHotkeyCapture() {
    await testHotkey();
  }

  async function confirmHotkey() {
    if (!capturedHotkey) return;
    set('push_to_talk_key', capturedHotkey);
    capturedHotkey = null;
  }

  function discardChanges() {
    local = { ...saved };
    Object.values(debounceTimers).forEach(clearTimeout);
  }

  async function resetDefaults() {
    local = { ...DEFAULT_CONFIG };
    await updateConfig({ ...DEFAULT_CONFIG });
    saved = { ...local };
  }

  function exportConfig() {
    const blob = new Blob([JSON.stringify(local, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'voice-prompt-config.json';
    a.click();
    URL.revokeObjectURL(url);
  }

  function importConfig() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      const text = await file.text();
      try {
        const parsed = JSON.parse(text) as Partial<Config>;
        local = { ...DEFAULT_CONFIG, ...parsed };
        await updateConfig(local);
        saved = { ...local };
      } catch {
        alert('Invalid config file');
      }
    };
    input.click();
  }

  // ── design tokens ──────────────────────────────────────────────────────────
  const card = 'bg-white dark:bg-gray-800/60 border border-gray-200 dark:border-gray-700 rounded-xl p-4 shadow-sm space-y-3';
  const sectionLabel = 'text-xs font-semibold uppercase tracking-wide text-gray-400 dark:text-gray-500';
  const selectCls = 'w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-colors';
  const chip = 'px-2 py-0.5 rounded-md bg-indigo-50 dark:bg-indigo-900/30 text-indigo-700 dark:text-indigo-300 text-xs font-mono';
  const btnSecondary = 'px-3 py-1.5 text-xs font-medium border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700/50 disabled:opacity-40 transition-colors';
</script>

<div class="space-y-3 max-w-sm">

  <!-- Hotkey (push-to-talk) -->
  <div class={card}>
    <p class={sectionLabel}>{t('hotkey')}</p>
    <div class="flex items-center gap-2 flex-wrap">
      <span class="px-3 py-1.5 bg-gray-100 dark:bg-gray-700 rounded-lg font-mono text-sm text-gray-800 dark:text-gray-200">
        {capturedHotkey ?? local.push_to_talk_key}
      </span>
      {#if armingHotkey}
        <span class="text-xs text-indigo-600 dark:text-indigo-400 animate-pulse font-medium">{t('press_hotkey')}</span>
      {:else}
        <button onclick={startHotkeyCapture} class={btnSecondary}>Change</button>
      {/if}
      {#if capturedHotkey}
        <button
          onclick={confirmHotkey}
          class="px-3 py-1.5 text-xs font-medium bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
        >
          Confirm
        </button>
      {/if}
    </div>
  </div>

  <!-- Whisper model + language -->
  <div class={card}>
    <p class={sectionLabel}>Whisper</p>

    <div class="space-y-1">
      <label for="model-select" class="text-xs text-gray-500 dark:text-gray-400">{t('model')}</label>
      <select
        id="model-select"
        value={local.whisper_model}
        onchange={(e) => set('whisper_model', (e.target as HTMLSelectElement).value)}
        class={selectCls}
      >
        {#each WHISPER_MODELS as m}
          <option value={m.id}>{m.label}</option>
        {/each}
      </select>
    </div>

    <div class="space-y-1">
      <label for="lang-select" class="text-xs text-gray-500 dark:text-gray-400">{t('language')}</label>
      <select
        id="lang-select"
        value={local.language}
        onchange={(e) => set('language', (e.target as HTMLSelectElement).value)}
        class={selectCls}
      >
        {#each LANGUAGES as l}
          <option value={l.id}>{l.label}</option>
        {/each}
      </select>
    </div>
  </div>

  <!-- VAD filter -->
  <div class="bg-white dark:bg-gray-800/60 border border-gray-200 dark:border-gray-700 rounded-xl px-4 py-3 shadow-sm">
    <label class="flex items-center gap-3 cursor-pointer">
      <input
        id="vad-toggle"
        type="checkbox"
        checked={local.vad_filter}
        onchange={(e) => set('vad_filter', (e.target as HTMLInputElement).checked)}
        class="w-4 h-4 accent-indigo-600 rounded"
      />
      <span class="text-sm font-medium text-gray-700 dark:text-gray-300">{t('vad')}</span>
    </label>
  </div>

  <!-- Compute type -->
  <div class={card}>
    <label for="compute-select" class={sectionLabel}>{t('compute')}</label>
    <select
      id="compute-select"
      value={local.compute_type}
      onchange={(e) => set('compute_type', (e.target as HTMLSelectElement).value)}
      class={selectCls}
    >
      {#each COMPUTE_TYPES as c}
        <option value={c.id}>{c.label}</option>
      {/each}
    </select>
  </div>

  <!-- Python interpreter -->
  <div class={card}>
    <label for="python-input" class={sectionLabel}>{t('python')}</label>
    <input
      id="python-input"
      type="text"
      value={local.python_bin}
      oninput={(e) => set('python_bin', (e.target as HTMLInputElement).value)}
      class="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 font-mono text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-colors"
    />
  </div>

  <!-- Status message -->
  {#if saveMsg}
    <p class="text-xs text-emerald-600 dark:text-emerald-400 px-1">{saveMsg}</p>
  {/if}

  <!-- Footer actions -->
  <div class="flex gap-2 flex-wrap border-t border-gray-200 dark:border-gray-700 pt-3">
    <button onclick={discardChanges} disabled={!dirty} class={btnSecondary}>{t('discard')}</button>
    <button onclick={resetDefaults} class={btnSecondary}>{t('reset')}</button>
    <button onclick={exportConfig} class={btnSecondary}>{t('export')}</button>
    <button onclick={importConfig} class={btnSecondary}>{t('import')}</button>
  </div>

</div>
