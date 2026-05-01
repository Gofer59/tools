<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  let previewText = $state('');
  let visible     = $state(false);

  let unlistenShow:    (() => void) | null = null;
  let unlistenHide:    (() => void) | null = null;
  let unlistenPartial: (() => void) | null = null;
  let unlistenFinal:   (() => void) | null = null;

  onMount(async () => {
    unlistenShow = await listen<{ text: string }>('show-overlay', (e) => {
      previewText = e.payload?.text ?? previewText;
      visible = true;
    });
    unlistenHide = await listen('hide-overlay', () => {
      visible = false;
      previewText = '';
    });
    unlistenPartial = await listen<{ text: string; seq: number }>('partial-transcript', (e) => {
      if (e.payload?.text != null) {
        previewText = e.payload.text;
        visible = true;
      }
    });
    unlistenFinal = await listen<{ text: string }>('final-transcript', (e) => {
      if (e.payload?.text != null) {
        previewText = e.payload.text;
      }
    });
  });

  onDestroy(() => {
    unlistenShow?.();
    unlistenHide?.();
    unlistenPartial?.();
    unlistenFinal?.();
  });
</script>

<div
  class="h-screen w-screen flex items-center justify-center px-4"
  style="background: transparent;"
>
  {#if visible}
    <div
      class="w-full px-4 py-2 rounded-xl shadow-2xl
             bg-black/80 backdrop-blur-sm
             border border-white/10
             flex items-center gap-2"
    >
      <span class="w-2 h-2 rounded-full bg-indigo-400 animate-pulse shrink-0"></span>
      <p class="text-white text-sm font-mono italic truncate flex-1">{previewText}</p>
    </div>
  {/if}
</div>
