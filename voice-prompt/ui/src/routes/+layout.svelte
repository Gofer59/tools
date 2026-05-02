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
    <div class="flex items-center gap-1.5 mr-2 text-indigo-600 dark:text-indigo-400">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5" aria-hidden="true">
        <path d="M8.25 4.5a3.75 3.75 0 1 1 7.5 0v8.25a3.75 3.75 0 1 1-7.5 0V4.5Z" />
        <path d="M6 10.5a.75.75 0 0 1 .75.75v1.5a5.25 5.25 0 1 0 10.5 0v-1.5a.75.75 0 0 1 1.5 0v1.5a6.751 6.751 0 0 1-6 6.709v2.291h3a.75.75 0 0 1 0 1.5h-7.5a.75.75 0 0 1 0-1.5h3v-2.291a6.751 6.751 0 0 1-6-6.709v-1.5A.75.75 0 0 1 6 10.5Z" />
      </svg>
      <span class="text-sm font-semibold text-gray-800 dark:text-gray-100">voice-prompt</span>
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
