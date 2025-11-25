<script lang="ts">
  import { api } from '../lib/api';
  import type { JiraIssue } from '../lib/types';

  export let show: boolean = false;
  export let onSelect: (issueKey: string | null) => void;
  export let onCancel: () => void;

  let availableIssues: JiraIssue[] = [];
  let loadingIssues = false;

  // Generate a consistent color for an issue key or time code
  function getTagColor(tag: string): { bg: string; text: string; shadow: string } {
    // Use a better hash function (DJB2) for better distribution
    let hash = 5381;
    for (let i = 0; i < tag.length; i++) {
      hash = ((hash << 5) + hash) + tag.charCodeAt(i); // hash * 33 + c
    }

    // Curated color palette (hues that look good together)
    // Using colors inspired by macOS system colors
    const colorPalette = [
      { hue: 211, name: 'Blue' },      // System blue
      { hue: 154, name: 'Green' },     // System green
      { hue: 349, name: 'Red' },       // System red
      { hue: 40, name: 'Orange' },     // System orange
      { hue: 281, name: 'Purple' },    // System purple
      { hue: 186, name: 'Teal' },      // Teal
      { hue: 320, name: 'Pink' },      // Pink
      { hue: 26, name: 'Brown' },      // Brown
      { hue: 200, name: 'Cyan' },      // Cyan
      { hue: 95, name: 'Lime' },       // Lime
      { hue: 265, name: 'Indigo' },    // Indigo
      { hue: 15, name: 'Coral' }       // Coral
    ];

    // Select a color from the palette based on hash
    const colorIndex = Math.abs(hash) % colorPalette.length;
    const hue = colorPalette[colorIndex].hue;

    // Use different saturation/lightness for light/dark mode
    const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

    if (isDark) {
      return {
        bg: `hsl(${hue}, 45%, 25%)`,
        text: `hsl(${hue}, 65%, 80%)`,
        shadow: `hsla(${hue}, 45%, 15%, 0.4)`
      };
    } else {
      return {
        bg: `hsl(${hue}, 50%, 92%)`,
        text: `hsl(${hue}, 70%, 40%)`,
        shadow: `hsla(${hue}, 50%, 70%, 0.15)`
      };
    }
  }

  // Load issues when shown
  $: if (show && availableIssues.length === 0) {
    loadIssues();
  }

  async function loadIssues() {
    loadingIssues = true;
    try {
      availableIssues = await api.getAllJiraIssues();
      // Sort: cached issues first (alphabetically), then fresh issues (alphabetically)
      availableIssues.sort((a, b) => {
        if (a.from_cache && !b.from_cache) return -1;
        if (!a.from_cache && b.from_cache) return 1;
        return a.key.localeCompare(b.key);
      });
    } catch (e) {
      const errorMsg = String(e);
      if (errorMsg.includes('not configured')) {
        alert('Please configure Jira in Settings first to see available issues.');
      } else {
        alert(`Failed to load issues: ${e}`);
      }
      console.error('Failed to load issues:', e);
      availableIssues = [];
      onCancel();
    } finally {
      loadingIssues = false;
    }
  }

  function handleSelect(issueKey: string | null) {
    onSelect(issueKey);
  }

  function handleOverlayClick() {
    onCancel();
  }
</script>

{#if show}
  <div
    class="issue-dropdown-overlay"
    on:click={handleOverlayClick}
    on:keydown={(e) => e.key === 'Escape' && onCancel()}
    role="presentation"
  >
    <div
      class="issue-dropdown"
      on:click={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      on:keydown={(e) => e.stopPropagation()}
    >
      <div class="issue-dropdown-header">Select Issue</div>

      {#if loadingIssues}
        <div class="issue-dropdown-loading">Loading issues...</div>
      {:else}
        <div class="issue-dropdown-item"
          on:click={() => handleSelect(null)}
          on:keydown={(e) => e.key === 'Enter' && handleSelect(null)}
          role="button"
          tabindex="0"
        >
          <span class="issue-dropdown-key no-issue">No issue</span>
        </div>
        {#each availableIssues.slice(0, 20) as issue}
          {@const colors = getTagColor(issue.key)}
          <div class="issue-dropdown-item"
            on:click={() => handleSelect(issue.key)}
            on:keydown={(e) => e.key === 'Enter' && handleSelect(issue.key)}
            role="button"
            tabindex="0"
          >
            <span
              class="issue-dropdown-key issue-tag"
              style="background: {colors.bg}; color: {colors.text}; box-shadow: inset 0 1px 2px {colors.shadow}, inset 0 0 1px {colors.shadow};"
            >
              {issue.key}
            </span>
            <span class="issue-dropdown-summary">{issue.summary}</span>
          </div>
        {/each}
      {/if}
    </div>
  </div>
{/if}

<style>
  .issue-dropdown-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.3);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .issue-dropdown {
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
    width: 320px;
    max-height: 400px;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .issue-dropdown-header {
    padding: 10px 12px;
    font-size: 13px;
    font-weight: 600;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-secondary);
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .issue-dropdown-loading {
    padding: 20px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .issue-dropdown-item {
    padding: 8px 12px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 3px;
    transition: background 0.1s;
    border-bottom: 1px solid rgba(0, 0, 0, 0.05);
  }

  .issue-dropdown-item:hover {
    background: rgba(0, 122, 255, 0.08);
  }

  .issue-dropdown-item:last-child {
    border-bottom: none;
  }

  .issue-dropdown-key {
    font-family: 'SF Mono', Monaco, monospace;
    font-size: 13px;
    font-weight: 600;
  }

  .issue-dropdown-key.issue-tag {
    display: inline-flex;
    align-items: center;
    padding: 4px 12px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 500;
    border: none;
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.4;
  }

  .issue-dropdown-key.no-issue {
    color: var(--text-secondary);
    font-style: italic;
  }

  .issue-dropdown-summary {
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (prefers-color-scheme: dark) {
    .issue-dropdown-item {
      border-bottom-color: rgba(255, 255, 255, 0.05);
    }
  }
</style>

