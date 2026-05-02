<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { config, DEFAULT_CONFIG, type Config } from '$lib/stores';
  import { getConfig, updateConfig, listenConfigApplied } from '$lib/tauri-config';
  import { testHotkey, listenHotkeyCaptured, listenHotkeyTestArmed } from '$lib/tauri-hotkey';
  import { t } from '$lib/i18n';

  // ── state ──────────────────────────────────────────────────────────────────
  let local = $state<Config>({ ...$config });
  let saved = $state<Config>({ ...$config });
  let dirty = $derived(JSON.stringify(local) !== JSON.stringify(saved));
  let armingWhich = $state<string | null>(null);
  let capturedKey = $state<string | null>(null);
  let capturedWhich = $state<string | null>(null);
  let saveMsg = $state('');
  let debounceTimers: Record<string, ReturnType<typeof setTimeout>> = {};

  let unlistenConfig: (() => void) | null = null;
  let unlistenCaptured: (() => void) | null = null;
  let unlistenArmed: (() => void) | null = null;

  const OCR_LANGUAGES = [
    { id: 'eng',     label: 'English' },
    { id: 'fra',     label: 'French' },
    { id: 'eng+fra', label: 'English + French' },
  ];

  const DELIVERY_MODES = [
    { id: 'clipboard', label: 'Clipboard only' },
    { id: 'type',      label: 'Type at cursor' },
    { id: 'both',      label: 'Clipboard + Type' },
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
  });

  onDestroy(() => {
    unlistenConfig?.();
    unlistenCaptured?.();
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

  async function startHotkeyCapture(which: string) {
    await testHotkey(which);
  }

  async function confirmHotkey() {
    if (!capturedKey || !capturedWhich) return;
    const field = capturedWhich as keyof Config;
    set(field, capturedKey);
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
    a.download = 'screen-ocr-config.json';
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
  const btnSecondary = 'px-3 py-1.5 text-xs font-medium border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700/50 disabled:opacity-40 transition-colors';
  const btnPrimary = 'px-3 py-1.5 text-xs font-medium bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors';
</script>

<div class="space-y-3 max-w-sm">

  <!-- Hotkey: Quick Capture -->
  <div class={card}>
    <p class={sectionLabel}>{t('hotkey_quick_capture')}</p>
    <div class="flex items-center gap-2 flex-wrap">
      <span class="px-3 py-1.5 bg-gray-100 dark:bg-gray-700 rounded-lg font-mono text-sm text-gray-800 dark:text-gray-200">
        {capturedWhich === 'hotkey_quick_capture' && capturedKey ? capturedKey : local.hotkey_quick_capture}
      </span>
      {#if armingWhich === 'hotkey_quick_capture'}
        <span class="text-xs text-indigo-600 dark:text-indigo-400 animate-pulse font-medium">{t('press_hotkey')}</span>
        <button onclick={cancelCapture} class={btnSecondary}>Cancel</button>
      {:else}
        <button onclick={() => startHotkeyCapture('hotkey_quick_capture')} class={btnSecondary}>Change</button>
      {/if}
      {#if capturedWhich === 'hotkey_quick_capture' && capturedKey}
        <button onclick={confirmHotkey} class={btnPrimary}>Confirm</button>
      {/if}
    </div>
  </div>

  <!-- Hotkey: Select Region -->
  <div class={card}>
    <p class={sectionLabel}>{t('hotkey_select_region')}</p>
    <div class="flex items-center gap-2 flex-wrap">
      <span class="px-3 py-1.5 bg-gray-100 dark:bg-gray-700 rounded-lg font-mono text-sm text-gray-800 dark:text-gray-200">
        {capturedWhich === 'hotkey_select_region' && capturedKey ? capturedKey : local.hotkey_select_region}
      </span>
      {#if armingWhich === 'hotkey_select_region'}
        <span class="text-xs text-indigo-600 dark:text-indigo-400 animate-pulse font-medium">{t('press_hotkey')}</span>
        <button onclick={cancelCapture} class={btnSecondary}>Cancel</button>
      {:else}
        <button onclick={() => startHotkeyCapture('hotkey_select_region')} class={btnSecondary}>Change</button>
      {/if}
      {#if capturedWhich === 'hotkey_select_region' && capturedKey}
        <button onclick={confirmHotkey} class={btnPrimary}>Confirm</button>
      {/if}
    </div>
  </div>

  <!-- Hotkey: Stop TTS -->
  <div class={card}>
    <p class={sectionLabel}>{t('hotkey_stop_tts')}</p>
    <div class="flex items-center gap-2 flex-wrap">
      <span class="px-3 py-1.5 bg-gray-100 dark:bg-gray-700 rounded-lg font-mono text-sm text-gray-800 dark:text-gray-200">
        {capturedWhich === 'hotkey_stop_tts' && capturedKey ? capturedKey : local.hotkey_stop_tts}
      </span>
      {#if armingWhich === 'hotkey_stop_tts'}
        <span class="text-xs text-indigo-600 dark:text-indigo-400 animate-pulse font-medium">{t('press_hotkey')}</span>
        <button onclick={cancelCapture} class={btnSecondary}>Cancel</button>
      {:else}
        <button onclick={() => startHotkeyCapture('hotkey_stop_tts')} class={btnSecondary}>Change</button>
      {/if}
      {#if capturedWhich === 'hotkey_stop_tts' && capturedKey}
        <button onclick={confirmHotkey} class={btnPrimary}>Confirm</button>
      {/if}
    </div>
  </div>

  <!-- OCR Language -->
  <div class={card}>
    <label for="ocr-lang-select" class={sectionLabel}>{t('ocr_language')}</label>
    <select
      id="ocr-lang-select"
      value={local.ocr_language}
      onchange={(e) => set('ocr_language', (e.target as HTMLSelectElement).value)}
      class={selectCls}
    >
      {#each OCR_LANGUAGES as l}
        <option value={l.id}>{l.label}</option>
      {/each}
    </select>
  </div>

  <!-- Delivery Mode -->
  <div class={card}>
    <label for="delivery-select" class={sectionLabel}>{t('delivery_mode')}</label>
    <select
      id="delivery-select"
      value={local.delivery_mode}
      onchange={(e) => set('delivery_mode', (e.target as HTMLSelectElement).value)}
      class={selectCls}
    >
      {#each DELIVERY_MODES as m}
        <option value={m.id}>{m.label}</option>
      {/each}
    </select>
  </div>

  <!-- TTS Voice -->
  <div class={card}>
    <label for="tts-voice-input" class={sectionLabel}>{t('tts_voice')}</label>
    <input
      id="tts-voice-input"
      type="text"
      value={local.tts_voice}
      oninput={(e) => set('tts_voice', (e.target as HTMLInputElement).value)}
      class="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 font-mono text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-colors"
    />
  </div>

  <!-- TTS Speed -->
  <div class={card}>
    <label for="tts-speed-input" class={sectionLabel}>{t('tts_speed')}</label>
    <div class="flex items-center gap-3">
      <input
        id="tts-speed-input"
        type="range"
        min="0.5"
        max="2.0"
        step="0.1"
        value={local.tts_speed}
        oninput={(e) => set('tts_speed', parseFloat((e.target as HTMLInputElement).value))}
        class="flex-1 accent-indigo-600"
      />
      <span class="text-sm font-mono text-gray-700 dark:text-gray-300 w-8 text-right">
        {local.tts_speed.toFixed(1)}
      </span>
    </div>
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
