<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { config, DEFAULT_CONFIG, type Config } from '$lib/stores';
  import { getConfig, updateConfig, listenConfigApplied } from '$lib/tauri-config';
  import { testHotkey, listenHotkeyTriggered, listenHotkeyTestArmed } from '$lib/tauri-hotkey';
  import { listCatalogModels, listLocalModels, downloadModel, type ModelEntry, type LocalModel } from '$lib/tauri-models';
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
  let localModels = $state<LocalModel[]>([]);
  let langFilter = $state('all');

  let unlistenConfig: (() => void) | null = null;
  let unlistenHotkey: (() => void) | null = null;
  let unlistenArmed: (() => void) | null = null;

  let installedIds = $derived(new Set(localModels.map((m) => m.id)));
  let availableLangs = $derived(['all', ...[...new Set(voiceCatalog.map((m) => m.language))].sort()]);
  let filteredVoices = $derived(
    voiceCatalog.filter((m) => langFilter === 'all' || m.language === langFilter),
  );
  let selectedModel = $derived(voiceCatalog.find((m) => m.id === local.voice) ?? null);
  let needsDownload = $derived(selectedModel !== null && !installedIds.has(local.voice));

  // ── lifecycle ──────────────────────────────────────────────────────────────
  onMount(async () => {
    const remote = await getConfig();
    local = { ...remote };
    saved = { ...remote };
    config.set(remote);

    [voiceCatalog, localModels] = await Promise.all([listCatalogModels(), listLocalModels()]);

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

  function formatBytes(b: number): string {
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(0)} KB`;
    return `${(b / (1024 * 1024)).toFixed(0)} MB`;
  }

  // ── design tokens (classes reused throughout) ──────────────────────────────
  const card = 'bg-white dark:bg-gray-800/60 border border-gray-200 dark:border-gray-700 rounded-xl p-4 shadow-sm space-y-3';
  const sectionLabel = 'text-xs font-semibold uppercase tracking-wide text-gray-400 dark:text-gray-500';
  const selectCls = 'w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-colors';
  const chip = 'px-2 py-0.5 rounded-md bg-indigo-50 dark:bg-indigo-900/30 text-indigo-700 dark:text-indigo-300 text-xs font-mono';
  const btnSecondary = 'px-3 py-1.5 text-xs font-medium border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700/50 disabled:opacity-40 transition-colors';
</script>

<div class="space-y-3 max-w-sm">

  <!-- Hotkey -->
  <div class={card}>
    <p class={sectionLabel}>{t('hotkey')}</p>
    <div class="flex items-center gap-2 flex-wrap">
      <span class="px-3 py-1.5 bg-gray-100 dark:bg-gray-700 rounded-lg font-mono text-sm text-gray-800 dark:text-gray-200">
        {capturedHotkey ?? local.hotkey}
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

  <!-- Voice picker -->
  <div class={card}>
    <p class={sectionLabel}>{t('voice')}</p>

    <!-- Language filter -->
    <div class="space-y-1">
      <label class="text-xs text-gray-500 dark:text-gray-400">{t('language')}</label>
      <select
        value={langFilter}
        onchange={(e) => (langFilter = (e.target as HTMLSelectElement).value)}
        class={selectCls}
      >
        {#each availableLangs as lang}
          <option value={lang}>{lang === 'all' ? 'All languages' : lang}</option>
        {/each}
      </select>
    </div>

    <!-- Voice dropdown -->
    <div class="space-y-1">
      <label class="text-xs text-gray-500 dark:text-gray-400">Model</label>
      <select
        value={local.voice}
        onchange={(e) => set('voice', (e.target as HTMLSelectElement).value)}
        class={selectCls}
      >
        {#each filteredVoices as m}
          <option value={m.id}>
            {installedIds.has(m.id) ? '✓' : '⬇'} {m.display_name} — {m.language}
          </option>
        {/each}
        {#if filteredVoices.length === 0}
          <option disabled value="">No voices for this language</option>
        {/if}
      </select>
    </div>

    <!-- Download notice for uninstalled selection -->
    {#if needsDownload && selectedModel}
      <div class="flex items-start gap-2.5 p-3 rounded-lg bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-700">
        <span class="text-amber-500 text-sm leading-none mt-0.5 shrink-0">⬇</span>
        <div class="flex-1 min-w-0">
          <p class="text-xs font-medium text-amber-700 dark:text-amber-300">Will download before first use</p>
          <p class="text-xs text-amber-600 dark:text-amber-400 mt-0.5">~{formatBytes(selectedModel.size_bytes)}</p>
        </div>
        <button
          onclick={() => downloadModel(local.voice)}
          class="shrink-0 px-2.5 py-1 text-xs font-medium border border-amber-400 dark:border-amber-600 text-amber-700 dark:text-amber-300 rounded-md hover:bg-amber-100 dark:hover:bg-amber-800/40 transition-colors"
        >
          {t('download')}
        </button>
      </div>
    {/if}
  </div>

  <!-- Speed -->
  <div class={card}>
    <div class="flex items-center justify-between">
      <p class={sectionLabel}>{t('speed')}</p>
      <span class={chip}>{local.speed.toFixed(2)}×</span>
    </div>
    <input
      type="range" min="0.5" max="2.0" step="0.05"
      value={local.speed}
      oninput={(e) => set('speed', parseFloat((e.target as HTMLInputElement).value))}
      class="w-full accent-indigo-600"
    />
    <div class="flex justify-between text-xs text-gray-400">
      <span>0.5×</span><span>2.0×</span>
    </div>
  </div>

  <!-- Noise scale -->
  <div class={card}>
    <div class="flex items-center justify-between">
      <p class={sectionLabel}>{t('noise')}</p>
      <span class={chip}>{local.noise_scale.toFixed(3)}</span>
    </div>
    <input
      type="range" min="0.0" max="1.0" step="0.01"
      value={local.noise_scale}
      oninput={(e) => set('noise_scale', parseFloat((e.target as HTMLInputElement).value))}
      class="w-full accent-indigo-600"
    />
    <div class="flex justify-between text-xs text-gray-400">
      <span>0.0</span><span>1.0</span>
    </div>
  </div>

  <!-- Noise width -->
  <div class={card}>
    <div class="flex items-center justify-between">
      <p class={sectionLabel}>{t('noise_w')}</p>
      <span class={chip}>{local.noise_w_scale.toFixed(2)}</span>
    </div>
    <input
      type="range" min="0.0" max="1.5" step="0.05"
      value={local.noise_w_scale}
      oninput={(e) => set('noise_w_scale', parseFloat((e.target as HTMLInputElement).value))}
      class="w-full accent-indigo-600"
    />
    <div class="flex justify-between text-xs text-gray-400">
      <span>0.0</span><span>1.5</span>
    </div>
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

  <!-- Test voice -->
  <div class={card}>
    <p class={sectionLabel}>{t('test_voice')}</p>
    <button
      onclick={previewVoice}
      class="px-4 py-2 text-sm font-medium bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 transition-colors"
    >
      {t('play')}
    </button>
  </div>

  <!-- Status message -->
  {#if saveMsg}
    <p class="text-xs text-emerald-600 dark:text-emerald-400 px-1">{saveMsg}</p>
  {/if}

  <!-- Footer actions -->
  <div class="flex gap-2 pt-1 flex-wrap border-t border-gray-200 dark:border-gray-700 pt-3">
    <button onclick={discardChanges} disabled={!dirty} class={btnSecondary}>{t('discard')}</button>
    <button onclick={resetDefaults} class={btnSecondary}>{t('reset')}</button>
    <button onclick={exportConfig} class={btnSecondary}>{t('export')}</button>
    <button onclick={importConfig} class={btnSecondary}>{t('import')}</button>
  </div>

</div>
