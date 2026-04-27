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

<nav>
  {#each tabs as tab}
    <a
      href={tab.href}
      class:active={activeTab === tab.id}
      onclick={() => (activeTab = tab.id)}
    >
      {tab.label}
    </a>
  {/each}
</nav>

<main>
  {@render children()}
</main>

<style>
  nav {
    display: flex;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #e2e8f0;
  }
  nav a {
    padding: 0.375rem 0.75rem;
    border-radius: 0.375rem;
    text-decoration: none;
    color: inherit;
    font-weight: 500;
  }
  nav a.active {
    background: #3b82f6;
    color: white;
  }
  main {
    padding: 1.5rem;
  }
</style>
