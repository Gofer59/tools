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
    { id: 'tiny', label: 'Tiny (~75 MB)' },
    { id: 'base', label: 'Base (~145 MB)' },
    { id: 'small', label: 'Small (~466 MB)' },
    { id: 'medium', label: 'Medium (~1.5 GB)' },
    { id: 'large-v3', label: 'Large v3 (~2.9 GB)' },
  ];

  const LANGUAGES = [
    { id: 'en', label: 'English' },
    { id: 'fr', label: 'French' },
    { id: 'de', label: 'German' },
    { id: 'es', label: 'Spanish' },
    { id: 'auto', label: 'Auto-detect' },
  ];

  const COMPUTE_TYPES = [
    { id: 'int8', label: 'int8 (fastest)' },
    { id: 'int8_float16', label: 'int8_float16' },
    { id: 'float16', label: 'float16' },
    { id: 'float32', label: 'float32 (most accurate)' },
  ];

  // ── lifecycle ──────────────────────────────────────────────────────────────
  onMount(async () => {
    const remote = await getConfig();
    local = { ...remote };
    saved = { ...remote };
    config.set(remote);

    unlistenConfig = await listenConfigApplied(({ field, value }) => {
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
      } catch (e) {
        // Revert the field on error
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
</script>

<div class="space-y-6 max-w-2xl">
  <h1 class="text-xl font-semibold">{t('settings')}</h1>

  <!-- Hotkey -->
  <section class="space-y-2">
    <p class="text-sm font-medium">{t('hotkey')}</p>
    <div class="flex items-center gap-2">
      <span class="px-3 py-1.5 bg-gray-100 dark:bg-gray-800 rounded font-mono text-sm">
        {capturedHotkey ?? local.push_to_talk_key}
      </span>
      {#if armingHotkey}
        <span class="text-sm text-blue-600 animate-pulse">{t('press_hotkey')}</span>
      {:else}
        <button
          onclick={startHotkeyCapture}
          class="px-3 py-1.5 text-sm border rounded hover:bg-gray-50 dark:hover:bg-gray-800"
        >
          Change
        </button>
      {/if}
      {#if capturedHotkey}
        <button
          onclick={confirmHotkey}
          class="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
        >
          Confirm
        </button>
      {/if}
    </div>
  </section>

  <!-- Whisper Model -->
  <section class="space-y-2">
    <label for="model-select" class="text-sm font-medium">{t('model')}</label>
    <select
      id="model-select"
      value={local.whisper_model}
      onchange={(e) => set('whisper_model', (e.target as HTMLSelectElement).value)}
      class="w-full max-w-xs px-3 py-2 border rounded text-sm bg-white dark:bg-gray-900"
    >
      {#each WHISPER_MODELS as m}
        <option value={m.id}>{m.label}</option>
      {/each}
    </select>
  </section>

  <!-- Language -->
  <section class="space-y-2">
    <label for="lang-select" class="text-sm font-medium">{t('language')}</label>
    <select
      id="lang-select"
      value={local.language}
      onchange={(e) => set('language', (e.target as HTMLSelectElement).value)}
      class="w-full max-w-xs px-3 py-2 border rounded text-sm bg-white dark:bg-gray-900"
    >
      {#each LANGUAGES as l}
        <option value={l.id}>{l.label}</option>
      {/each}
    </select>
  </section>

  <!-- VAD filter -->
  <section class="flex items-center gap-3">
    <input
      id="vad-toggle"
      type="checkbox"
      checked={local.vad_filter}
      onchange={(e) => set('vad_filter', (e.target as HTMLInputElement).checked)}
      class="w-4 h-4 accent-blue-600"
    />
    <label for="vad-toggle" class="text-sm font-medium">{t('vad')}</label>
  </section>

  <!-- Python bin -->
  <section class="space-y-2">
    <label for="python-input" class="text-sm font-medium">{t('python')}</label>
    <input
      id="python-input"
      type="text"
      value={local.python_bin}
      oninput={(e) => set('python_bin', (e.target as HTMLInputElement).value)}
      class="w-full max-w-xs px-3 py-2 border rounded text-sm font-mono bg-white dark:bg-gray-900"
    />
  </section>

  <!-- Compute type -->
  <section class="space-y-2">
    <label for="compute-select" class="text-sm font-medium">{t('compute')}</label>
    <select
      id="compute-select"
      value={local.compute_type}
      onchange={(e) => set('compute_type', (e.target as HTMLSelectElement).value)}
      class="w-full max-w-xs px-3 py-2 border rounded text-sm bg-white dark:bg-gray-900"
    >
      {#each COMPUTE_TYPES as c}
        <option value={c.id}>{c.label}</option>
      {/each}
    </select>
  </section>

  <!-- Status / save message -->
  {#if saveMsg}
    <p class="text-sm text-green-600">{saveMsg}</p>
  {/if}

  <!-- Footer actions -->
  <div class="flex gap-2 pt-4 border-t">
    <button
      onclick={discardChanges}
      disabled={!dirty}
      class="px-3 py-1.5 text-sm border rounded hover:bg-gray-50 dark:hover:bg-gray-800 disabled:opacity-40"
    >
      {t('discard')}
    </button>
    <button
      onclick={resetDefaults}
      class="px-3 py-1.5 text-sm border rounded hover:bg-gray-50 dark:hover:bg-gray-800"
    >
      {t('reset')}
    </button>
    <button
      onclick={exportConfig}
      class="px-3 py-1.5 text-sm border rounded hover:bg-gray-50 dark:hover:bg-gray-800"
    >
      {t('export')}
    </button>
    <button
      onclick={importConfig}
      class="px-3 py-1.5 text-sm border rounded hover:bg-gray-50 dark:hover:bg-gray-800"
    >
      {t('import')}
    </button>
  </div>
</div>
