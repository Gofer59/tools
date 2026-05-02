<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { config, DEFAULT_CONFIG, type Config } from '$lib/stores';
  import { getConfig, updateConfig, listenConfigApplied } from '$lib/tauri-config';
  import { testHotkey, listenHotkeyCaptured, listenHotkeyTestArmed } from '$lib/tauri-hotkey';
  import { startOverlay, stopOverlay, isOverlayRunning, listenOverlayState } from '$lib/tauri-overlay';
  import { t } from '$lib/i18n';

  // ── state ──────────────────────────────────────────────────────────────────
  let local = $state<Config>({ ...$config });
  let saved = $state<Config>({ ...$config });
  let dirty = $derived(JSON.stringify(local) !== JSON.stringify(saved));
  let armingWhich = $state<string | null>(null);
  let capturedKey = $state<string | null>(null);
  let capturedWhich = $state<string | null>(null);
  let saveMsg = $state('');
  let overlayRunning = $state(false);
  let debounceTimers: Record<string, ReturnType<typeof setTimeout>> = {};

  let unlistenConfig: (() => void) | null = null;
  let unlistenCaptured: (() => void) | null = null;
  let unlistenArmed: (() => void) | null = null;
  let unlistenOverlay: (() => void) | null = null;

  // ── lifecycle ──────────────────────────────────────────────────────────────
  onMount(async () => {
    const remote = await getConfig();
    local = { ...remote };
    saved = { ...remote };
    config.set(remote);

    overlayRunning = await isOverlayRunning();

    unlistenConfig = await listenConfigApplied(() => {
      saveMsg = '✓ saved';
      setTimeout(() => (saveMsg = ''), 2000);
    });

    unlistenCaptured = await listenHotkeyCaptured((e) => {
      if (armingWhich === e.which) {
        capturedKey = e.captured;
        capturedWhich = e.which;
        armingWhich = null;
      }
    });

    unlistenArmed = await listenHotkeyTestArmed((e) => {
      armingWhich = e.which;
      capturedKey = null;
      capturedWhich = null;
    });

    unlistenOverlay = await listenOverlayState((e) => {
      overlayRunning = e.running;
    });
  });

  onDestroy(() => {
    unlistenConfig?.();
    unlistenCaptured?.();
    unlistenArmed?.();
    unlistenOverlay?.();
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

  async function startHotkeyCapture(which: string) {
    await testHotkey(which);
  }

  async function confirmHotkey() {
    if (!capturedKey || !capturedWhich) return;
    // Only string-typed hotkey fields are valid targets for key capture.
    if (capturedWhich !== 'region_select_hotkey' && capturedWhich !== 'toggle_on_top_hotkey') return;
    set(capturedWhich, capturedKey);
    capturedKey = null;
    capturedWhich = null;
  }

  function cancelCapture() {
    armingWhich = null;
    capturedKey = null;
    capturedWhich = null;
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
    a.download = 'threshold-filter-config.json';
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
  const btnSecondary = 'px-3 py-1.5 text-xs font-medium border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700/50 disabled:opacity-40 transition-colors';
  const btnPrimary = 'px-3 py-1.5 text-xs font-medium bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors';
  const btnDanger = 'px-3 py-1.5 text-xs font-medium bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors';
</script>

<div class="space-y-3 max-w-sm">

  <!-- Overlay status + controls -->
  <div class={card}>
    <p class={sectionLabel}>{t('overlay_status')}</p>
    <div class="flex items-center gap-3">
      <span class="flex items-center gap-1.5 text-sm">
        <span class="inline-block w-2 h-2 rounded-full {overlayRunning ? 'bg-emerald-500' : 'bg-gray-400'}"></span>
        {overlayRunning ? 'Running' : 'Stopped'}
      </span>
      {#if overlayRunning}
        <button onclick={() => stopOverlay()} class={btnDanger}>Stop</button>
      {:else}
        <button onclick={() => startOverlay()} class={btnPrimary}>Start</button>
      {/if}
    </div>
  </div>

  <!-- Hotkey: Select Region -->
  <div class={card}>
    <p class={sectionLabel}>{t('region_select_hotkey')}</p>
    <div class="flex items-center gap-2 flex-wrap">
      <span class="px-3 py-1.5 bg-gray-100 dark:bg-gray-700 rounded-lg font-mono text-sm text-gray-800 dark:text-gray-200">
        {capturedWhich === 'region_select_hotkey' && capturedKey ? capturedKey : local.region_select_hotkey}
      </span>
      {#if armingWhich === 'region_select_hotkey'}
        <span class="text-xs text-indigo-600 dark:text-indigo-400 animate-pulse font-medium">{t('press_hotkey')}</span>
        <button onclick={cancelCapture} class={btnSecondary}>Cancel</button>
      {:else}
        <button onclick={() => startHotkeyCapture('region_select_hotkey')} class={btnSecondary}>Change</button>
      {/if}
      {#if capturedWhich === 'region_select_hotkey' && capturedKey}
        <button onclick={confirmHotkey} class={btnPrimary}>Confirm</button>
      {/if}
    </div>
  </div>

  <!-- Hotkey: Toggle Always-on-Top -->
  <div class={card}>
    <p class={sectionLabel}>{t('toggle_on_top_hotkey')}</p>
    <div class="flex items-center gap-2 flex-wrap">
      <span class="px-3 py-1.5 bg-gray-100 dark:bg-gray-700 rounded-lg font-mono text-sm text-gray-800 dark:text-gray-200">
        {capturedWhich === 'toggle_on_top_hotkey' && capturedKey ? capturedKey : local.toggle_on_top_hotkey}
      </span>
      {#if armingWhich === 'toggle_on_top_hotkey'}
        <span class="text-xs text-indigo-600 dark:text-indigo-400 animate-pulse font-medium">{t('press_hotkey')}</span>
        <button onclick={cancelCapture} class={btnSecondary}>Cancel</button>
      {:else}
        <button onclick={() => startHotkeyCapture('toggle_on_top_hotkey')} class={btnSecondary}>Change</button>
      {/if}
      {#if capturedWhich === 'toggle_on_top_hotkey' && capturedKey}
        <button onclick={confirmHotkey} class={btnPrimary}>Confirm</button>
      {/if}
    </div>
  </div>

  <!-- Default threshold -->
  <div class={card}>
    <label for="threshold-input" class={sectionLabel}>{t('default_threshold')}</label>
    <div class="flex items-center gap-3">
      <input
        id="threshold-input"
        type="range"
        min="0"
        max="255"
        step="1"
        value={local.default_threshold}
        oninput={(e) => set('default_threshold', parseInt((e.target as HTMLInputElement).value, 10))}
        class="flex-1 accent-indigo-600"
      />
      <span class="text-sm font-mono text-gray-700 dark:text-gray-300 w-8 text-right">
        {local.default_threshold}
      </span>
    </div>
  </div>

  <!-- Checkboxes -->
  <div class={card + ' space-y-2'}>
    <label class="flex items-center gap-2 cursor-pointer select-none">
      <input
        type="checkbox"
        checked={local.default_invert}
        onchange={(e) => set('default_invert', (e.target as HTMLInputElement).checked)}
        class="w-4 h-4 accent-indigo-600"
      />
      <span class="text-sm text-gray-800 dark:text-gray-200">{t('default_invert')}</span>
    </label>
    <label class="flex items-center gap-2 cursor-pointer select-none">
      <input
        type="checkbox"
        checked={local.default_always_on_top}
        onchange={(e) => set('default_always_on_top', (e.target as HTMLInputElement).checked)}
        class="w-4 h-4 accent-indigo-600"
      />
      <span class="text-sm text-gray-800 dark:text-gray-200">{t('default_always_on_top')}</span>
    </label>
    <label class="flex items-center gap-2 cursor-pointer select-none">
      <input
        type="checkbox"
        checked={local.auto_start_overlay}
        onchange={(e) => set('auto_start_overlay', (e.target as HTMLInputElement).checked)}
        class="w-4 h-4 accent-indigo-600"
      />
      <span class="text-sm text-gray-800 dark:text-gray-200">{t('auto_start_overlay')}</span>
    </label>
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
