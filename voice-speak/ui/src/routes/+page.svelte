<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { config, DEFAULT_CONFIG, type Config } from '$lib/stores';
  import { getConfig, updateConfig, listenConfigApplied } from '$lib/tauri-config';
  import { testHotkey, listenHotkeyTriggered, listenHotkeyTestArmed } from '$lib/tauri-hotkey';
  import { listCatalogModels, type ModelEntry } from '$lib/tauri-models';
  import { t } from '$lib/i18n';

  // ── state ──────────────────────────────────────────────────────────────────
  let local = $state<Config>({ ...$config });
  let saved = $state<Config>({ ...$config });
  let dirty = $derived(JSON.stringify(local) !== JSON.stringify(saved));
  let capturedHotkey = $state<string | null>(null);
  let armingHotkey = $state(false);
  let saveMsg = $state('');
  let debounceTimers: Record<string, ReturnType<typeof setTimeout>> = {};

  let voiceCatalog = $state<ModelEntry[]>([]);
  let voiceFilter = $state('');

  let unlistenConfig: (() => void) | null = null;
  let unlistenHotkey: (() => void) | null = null;
  let unlistenArmed: (() => void) | null = null;

  let filteredVoices = $derived(
    voiceCatalog.filter(
      (m) =>
        voiceFilter === '' ||
        m.language.toLowerCase().includes(voiceFilter.toLowerCase()) ||
        m.display_name.toLowerCase().includes(voiceFilter.toLowerCase()),
    ),
  );

  // ── lifecycle ──────────────────────────────────────────────────────────────
  onMount(async () => {
    const remote = await getConfig();
    local = { ...remote };
    saved = { ...remote };
    config.set(remote);

    voiceCatalog = await listCatalogModels();

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
    set('hotkey', capturedHotkey);
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
    a.download = 'voice-speak-config.json';
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

  async function previewVoice() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('preview_voice', { text: 'The quick brown fox.' });
    } catch (e) {
      console.log('preview_voice not yet wired', e);
    }
  }
</script>

<div class="space-y-6 max-w-2xl">
  <h1 class="text-xl font-semibold">{t('settings')}</h1>

  <!-- Hotkey -->
  <section class="space-y-2">
    <p class="text-sm font-medium">{t('hotkey')}</p>
    <div class="flex items-center gap-2">
      <span class="px-3 py-1.5 bg-gray-100 dark:bg-gray-800 rounded font-mono text-sm">
        {capturedHotkey ?? local.hotkey}
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

  <!-- Voice picker -->
  <section class="space-y-2">
    <label for="voice-filter" class="text-sm font-medium">{t('voice')}</label>
    <input
      id="voice-filter"
      type="text"
      placeholder="Filter by language or name…"
      bind:value={voiceFilter}
      class="w-full max-w-xs px-3 py-2 border rounded text-sm bg-white dark:bg-gray-900 mb-1"
    />
    <select
      id="voice-select"
      value={local.voice}
      onchange={(e) => set('voice', (e.target as HTMLSelectElement).value)}
      class="w-full max-w-xs px-3 py-2 border rounded text-sm bg-white dark:bg-gray-900"
      size="5"
    >
      {#each filteredVoices as m}
        <option value={m.id}>{m.display_name} — {m.language}</option>
      {/each}
      {#if filteredVoices.length === 0}
        <option disabled value="">No voices matched</option>
      {/if}
    </select>
    <p class="text-xs text-gray-500">Current: <span class="font-mono">{local.voice}</span></p>
  </section>

  <!-- Speed slider -->
  <section class="space-y-2">
    <label for="speed-slider" class="text-sm font-medium">
      {t('speed')} — <span class="font-mono">{local.speed.toFixed(2)}</span>
    </label>
    <input
      id="speed-slider"
      type="range"
      min="0.5"
      max="2.0"
      step="0.05"
      value={local.speed}
      oninput={(e) => set('speed', parseFloat((e.target as HTMLInputElement).value))}
      class="w-full max-w-xs accent-blue-600"
    />
    <div class="flex justify-between text-xs text-gray-400 max-w-xs">
      <span>0.5×</span><span>2.0×</span>
    </div>
  </section>

  <!-- Noise scale slider -->
  <section class="space-y-2">
    <label for="noise-slider" class="text-sm font-medium">
      {t('noise')} — <span class="font-mono">{local.noise_scale.toFixed(3)}</span>
    </label>
    <input
      id="noise-slider"
      type="range"
      min="0.0"
      max="1.0"
      step="0.01"
      value={local.noise_scale}
      oninput={(e) => set('noise_scale', parseFloat((e.target as HTMLInputElement).value))}
      class="w-full max-w-xs accent-blue-600"
    />
    <div class="flex justify-between text-xs text-gray-400 max-w-xs">
      <span>0.0</span><span>1.0</span>
    </div>
  </section>

  <!-- Noise width slider -->
  <section class="space-y-2">
    <label for="noise-w-slider" class="text-sm font-medium">
      {t('noise_w')} — <span class="font-mono">{local.noise_w_scale.toFixed(2)}</span>
    </label>
    <input
      id="noise-w-slider"
      type="range"
      min="0.0"
      max="1.5"
      step="0.05"
      value={local.noise_w_scale}
      oninput={(e) => set('noise_w_scale', parseFloat((e.target as HTMLInputElement).value))}
      class="w-full max-w-xs accent-blue-600"
    />
    <div class="flex justify-between text-xs text-gray-400 max-w-xs">
      <span>0.0</span><span>1.5</span>
    </div>
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

  <!-- Test voice -->
  <section class="space-y-2">
    <p class="text-sm font-medium">{t('test_voice')}</p>
    <button
      onclick={previewVoice}
      class="px-3 py-1.5 text-sm bg-green-600 text-white rounded hover:bg-green-700"
    >
      {t('play')}
    </button>
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
