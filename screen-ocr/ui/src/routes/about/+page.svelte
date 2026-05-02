<script lang="ts">
  import { locale, strings } from '$lib/i18n';
  import { t } from '$lib/i18n';

  const VERSION = '0.1.0';
  const LICENSE = 'MIT';
  const REPO = 'https://github.com/Gofer59/tools';
</script>

<div class="space-y-6 max-w-xl">
  <h1 class="text-xl font-semibold">screen-ocr</h1>

  <p class="text-sm text-gray-600 dark:text-gray-400">
    Hotkey OCR + TTS for screen regions. Hold F10 to select a region, F9 to re-capture.
  </p>

  <dl class="space-y-3 text-sm">
    <div class="flex gap-4">
      <dt class="font-medium w-24">{t('version')}</dt>
      <dd class="text-gray-600 dark:text-gray-400">{VERSION}</dd>
    </div>
    <div class="flex gap-4">
      <dt class="font-medium w-24">{t('license')}</dt>
      <dd class="text-gray-600 dark:text-gray-400">{LICENSE}</dd>
    </div>
    <div class="flex gap-4">
      <dt class="font-medium w-24">{t('repo')}</dt>
      <dd>
        <a href={REPO} class="text-blue-600 hover:underline break-all">{REPO}</a>
      </dd>
    </div>
  </dl>

  <p class="text-sm text-gray-500">{t('no_telemetry')}</p>

  <div class="pt-2">
    <p class="text-sm font-medium mb-2">Language / Langue</p>
    <div class="flex gap-2">
      {#each Object.keys(strings) as lang}
        <button
          onclick={() => locale.set(lang as 'en' | 'fr')}
          class="px-3 py-1.5 text-sm border rounded hover:bg-gray-50 dark:hover:bg-gray-800"
          class:bg-blue-600={$locale === lang}
          class:text-white={$locale === lang}
        >
          {lang.toUpperCase()}
        </button>
      {/each}
    </div>
  </div>
</div>
