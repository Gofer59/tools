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
  <nav class="flex gap-1 px-3 py-2.5 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 shrink-0">
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
