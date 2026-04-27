<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { downloadProgress } from '$lib/stores';
  import {
    listCatalogModels,
    listLocalModels,
    downloadModel,
    cancelDownload,
    deleteLocalModel,
    listenDownloadProgress,
    listenDownloadComplete,
    listenDownloadError,
    type ModelEntry,
    type LocalModel,
  } from '$lib/tauri-models';
  import { t } from '$lib/i18n';

  let catalog = $state<ModelEntry[]>([]);
  let local = $state<LocalModel[]>([]);
  let loading = $state(true);
  let confirmDelete = $state<string | null>(null);

  let unlistenProgress: (() => void) | null = null;
  let unlistenComplete: (() => void) | null = null;
  let unlistenError: (() => void) | null = null;

  function isLocal(id: string): boolean {
    return local.some((m) => m.id === id);
  }

  function isDownloading(id: string): boolean {
    return id in $downloadProgress;
  }

  function formatBytes(n: number): string {
    if (n < 1_000_000) return `${(n / 1_000).toFixed(0)} KB`;
    if (n < 1_000_000_000) return `${(n / 1_000_000).toFixed(0)} MB`;
    return `${(n / 1_000_000_000).toFixed(1)} GB`;
  }

  function formatSpeed(bps: number): string {
    return `${(bps / 1_000).toFixed(0)} KB/s`;
  }

  onMount(async () => {
    [catalog, local] = await Promise.all([listCatalogModels(), listLocalModels()]);
    loading = false;

    unlistenProgress = await listenDownloadProgress((e) => {
      downloadProgress.update((d) => ({
        ...d,
        [e.id]: { bytes: e.bytes, total: e.total, speed_bps: e.speed_bps },
      }));
    });

    unlistenComplete = await listenDownloadComplete(async (e) => {
      downloadProgress.update((d) => {
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const { [e.id]: _unused, ...rest } = d;
        return rest;
      });
      local = await listLocalModels();
    });

    unlistenError = await listenDownloadError((e) => {
      downloadProgress.update((d) => {
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const { [e.id]: _unused, ...rest } = d;
        return rest;
      });
      alert(`Download failed: ${e.message}`);
    });
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenComplete?.();
    unlistenError?.();
  });

  async function startDownload(id: string) {
    await downloadModel(id);
  }

  async function cancel(id: string) {
    await cancelDownload(id);
  }

  async function confirmAndDelete(id: string) {
    await deleteLocalModel(id);
    local = await listLocalModels();
    confirmDelete = null;
  }
</script>

<div class="space-y-4 max-w-3xl">
  <h1 class="text-xl font-semibold">{t('models')}</h1>

  {#if loading}
    <p class="text-sm text-gray-500">Loading…</p>
  {:else}
    <table class="w-full text-sm border-collapse">
      <thead>
        <tr class="border-b text-left">
          <th class="py-2 pr-4 font-medium">Name</th>
          <th class="py-2 pr-4 font-medium">Language</th>
          <th class="py-2 pr-4 font-medium">Size</th>
          <th class="py-2 pr-4 font-medium">License</th>
          <th class="py-2">Status</th>
        </tr>
      </thead>
      <tbody>
        {#each catalog as m}
          {@const prog = $downloadProgress[m.id]}
          <tr class="border-b hover:bg-gray-50 dark:hover:bg-gray-800/50">
            <td class="py-2 pr-4 font-mono">{m.display_name}</td>
            <td class="py-2 pr-4 text-gray-600 dark:text-gray-400">{m.language}</td>
            <td class="py-2 pr-4 text-gray-600 dark:text-gray-400">{formatBytes(m.size_bytes)}</td>
            <td class="py-2 pr-4 text-gray-600 dark:text-gray-400">{m.license}</td>
            <td class="py-2">
              {#if prog}
                <div class="flex items-center gap-2">
                  <div class="w-24 h-1.5 bg-gray-200 rounded">
                    <div
                      class="h-1.5 bg-blue-500 rounded"
                      style="width: {prog.total ? Math.round((prog.bytes / prog.total) * 100) : 0}%"
                    ></div>
                  </div>
                  <span class="text-xs text-gray-500">{formatSpeed(prog.speed_bps)}</span>
                  <button onclick={() => cancel(m.id)} class="text-xs text-red-500 hover:underline">
                    {t('cancel')}
                  </button>
                </div>
              {:else if isLocal(m.id)}
                <div class="flex items-center gap-2">
                  <span class="text-xs text-green-600 font-medium">{t('downloaded')}</span>
                  {#if confirmDelete === m.id}
                    <button
                      onclick={() => confirmAndDelete(m.id)}
                      class="text-xs text-red-500 hover:underline">{t('delete')}</button
                    >
                    <button
                      onclick={() => (confirmDelete = null)}
                      class="text-xs text-gray-400 hover:underline">{t('cancel')}</button
                    >
                  {:else}
                    <button
                      onclick={() => (confirmDelete = m.id)}
                      class="text-xs text-gray-400 hover:underline">{t('delete')}</button
                    >
                  {/if}
                </div>
              {:else}
                <button
                  onclick={() => startDownload(m.id)}
                  class="px-2 py-1 text-xs border rounded hover:bg-gray-50 dark:hover:bg-gray-800"
                >
                  {t('download')}
                </button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>
