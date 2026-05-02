<script lang="ts">
  import '../app.css';

  let { children } = $props();

  let activeTab = $state('settings');

  const tabs = [
    { id: 'settings', label: 'Settings', href: '/' },
    { id: 'models', label: 'Models', href: '/models' },
    { id: 'about', label: 'About', href: '/about' },
  ];
</script>

<div class="flex flex-col min-h-screen bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-gray-100">
  <nav class="flex items-center gap-1 px-3 py-2.5 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 shrink-0">
    <div class="flex items-center gap-1.5 mr-2 text-emerald-600 dark:text-emerald-400">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5" aria-hidden="true">
        <path d="M13.5 4.06c0-1.336-1.616-2.005-2.56-1.06l-4.5 4.5H4.508c-1.141 0-2.318.664-2.66 1.905A9.76 9.76 0 0 0 1.5 12c0 .898.121 1.768.35 2.595.341 1.24 1.518 1.905 2.659 1.905h1.93l4.5 4.5c.945.945 2.561.276 2.561-1.06V4.06Z" />
        <path d="M18.584 5.106a.75.75 0 0 1 1.06 0c3.808 3.807 3.808 9.98 0 13.788a.75.75 0 0 1-1.06-1.06 8.25 8.25 0 0 0 0-11.668.75.75 0 0 1 0-1.06Z" />
        <path d="M15.932 7.757a.75.75 0 0 1 1.061 0 6 6 0 0 1 0 8.486.75.75 0 0 1-1.06-1.061 4.5 4.5 0 0 0 0-6.364.75.75 0 0 1 0-1.06Z" />
      </svg>
      <span class="text-sm font-semibold text-gray-800 dark:text-gray-100">voice-speak</span>
    </div>
    <div class="w-px h-4 bg-gray-200 dark:bg-gray-600 mr-1"></div>
    {#each tabs as tab}
      <a
        href={tab.href}
        onclick={() => (activeTab = tab.id)}
        class="px-3 py-1.5 rounded-lg text-sm font-medium no-underline transition-colors {activeTab === tab.id
          ? 'bg-indigo-600 text-white'
          : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700/50'}"
      >
        {tab.label}
      </a>
    {/each}
  </nav>

  <main class="flex-1 overflow-y-auto p-4">
    {@render children()}
  </main>
</div>
