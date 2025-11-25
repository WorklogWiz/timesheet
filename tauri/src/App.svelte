<script lang="ts">
  import "./app.css"; // Tailwind + Skeleton styles
  import { onMount } from "svelte";
  import { writable } from "svelte/store";
  import { initTheme, theme } from "./lib/theme";
  import { initSleepMonitor } from "./lib/sleepMonitor";
  import type { Route } from "./lib/types";

  // Import views
  import SessionsView from "./views/SessionsView.svelte";
  import WeekView from "./views/WeekView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import SleepDialog from "./components/SleepDialog.svelte";

  // Router state
  const currentRoute = writable<Route>("/sessions");

  // Initialize theme and sleep monitor on mount
  onMount(() => {
    console.log("App.svelte mounted");
    initTheme();
    initSleepMonitor();

    // Handle hash-based routing
    function handleRoute() {
      const hash = window.location.hash.slice(1) as Route;
      console.log("Route changed to:", hash);
      if (hash === "/sessions" || hash === "/week" || hash === "/settings") {
        currentRoute.set(hash);
      } else {
        currentRoute.set("/sessions");
      }
    }

    // Initial route
    handleRoute();

    // Listen for hash changes
    window.addEventListener("hashchange", handleRoute);

    return () => {
      window.removeEventListener("hashchange", handleRoute);
    };
  });
</script>

<main class="app" data-theme={$theme}>
  {#if $currentRoute === "/sessions"}
    <SessionsView />
  {:else if $currentRoute === "/week"}
    <WeekView />
  {:else if $currentRoute === "/settings"}
    <SettingsView />
  {/if}
</main>

<SleepDialog />

<style>
  :global(:root) {
    /* Light theme (macOS/Windows light) */
    --bg-primary: #ffffff;
    --bg-secondary: #f5f5f7;
    --text-primary: #1d1d1f;
    --text-secondary: #6e6e73;
    --border-color: #d2d2d7;
    --hover-bg: #f5f5f7;
    --active-bg: #e8e8ed;
  }

  :global([data-theme="dark"]) {
    /* Dark theme (macOS/Windows dark) */
    --bg-primary: #1e1e1e;
    --bg-secondary: #2d2d2d;
    --text-primary: #ffffff;
    --text-secondary: #b3b3b3;
    --border-color: #3d3d3d;
    --hover-bg: #2d2d2d;
    --active-bg: #3d3d3d;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
      Helvetica, Arial, sans-serif;
    background-color: var(--bg-secondary);
    color: var(--text-primary);
    font-size: 14px;
  }

  .app {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  :global(button) {
    padding: 8px 16px;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    background: var(--bg-primary);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.2s;
  }

  :global(button:hover) {
    background: var(--hover-bg);
    border-color: var(--text-secondary);
  }

  :global(button:active) {
    transform: scale(0.98);
  }

  :global(button:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
