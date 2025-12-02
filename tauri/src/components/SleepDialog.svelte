<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import { handleSleepResponse } from '../lib/sleepMonitor';

  let isVisible = false;
  let sleepDuration = '';
  let issueKey: string | null = null;
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    unlisten = await listen('show-sleep-dialog', (event: any) => {
      sleepDuration = event.payload.sleepDuration;
      issueKey = event.payload.issueKey;
      isVisible = true;
    });
  });

  onDestroy(() => {
    if (unlisten) {
      unlisten();
    }
  });

  async function handleStop() {
    await handleSleepResponse('stop');
    isVisible = false;
  }

  async function handleContinue() {
    await handleSleepResponse('continue');
    isVisible = false;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      handleContinue();
    }
  }
</script>

{#if isVisible}
  <div class="sleep-dialog-overlay" role="presentation" on:click={handleContinue} on:keydown={handleKeydown}>
    <div class="sleep-dialog" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation on:keydown|stopPropagation>
      <div class="dialog-icon">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z"/>
          <path d="M12 6v6l4 2"/>
        </svg>
      </div>

      <h2>System Wake Detected</h2>

      <p class="sleep-message">
        Your computer was asleep for <strong>{sleepDuration}</strong>.
      </p>

      {#if issueKey}
        <p class="timer-info">
          You have an active timer running for <span class="issue-key">{issueKey}</span>
        </p>
      {:else}
        <p class="timer-info">
          You have an active timer running.
        </p>
      {/if}

      <p class="question">What would you like to do?</p>

      <div class="button-group">
        <button class="btn btn-secondary" on:click={handleStop}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
          </svg>
          Stop Timer
        </button>

        <button class="btn btn-primary" on:click={handleContinue}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polygon points="5 3 19 12 5 21 5 3"/>
          </svg>
          Continue
        </button>
      </div>

      <p class="hint">Press ESC to continue</p>
    </div>
  </div>
{/if}

<style>
  .sleep-dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
    animation: fadeIn 0.2s ease;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .sleep-dialog {
    background: white;
    border-radius: 12px;
    padding: 32px;
    max-width: 480px;
    width: 90%;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
    text-align: center;
    animation: slideUp 0.3s ease;
  }

  @media (prefers-color-scheme: dark) {
    .sleep-dialog {
      background: #2a2a2a;
      color: #ffffff;
    }
  }

  @keyframes slideUp {
    from {
      transform: translateY(20px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  .dialog-icon {
    color: #007aff;
    margin-bottom: 16px;
  }

  h2 {
    font-size: 24px;
    font-weight: 600;
    margin: 0 0 16px;
    color: #000;
  }

  @media (prefers-color-scheme: dark) {
    h2 {
      color: #fff;
    }
  }

  .sleep-message {
    font-size: 16px;
    margin: 12px 0;
    color: #333;
  }

  @media (prefers-color-scheme: dark) {
    .sleep-message {
      color: #ddd;
    }
  }

  .timer-info {
    font-size: 15px;
    margin: 12px 0;
    color: #666;
  }

  @media (prefers-color-scheme: dark) {
    .timer-info {
      color: #999;
    }
  }

  .issue-key {
    font-weight: 600;
    color: #007aff;
    background: rgba(0, 122, 255, 0.1);
    padding: 2px 8px;
    border-radius: 4px;
  }

  .question {
    font-size: 16px;
    font-weight: 500;
    margin: 24px 0 20px;
    color: #000;
  }

  @media (prefers-color-scheme: dark) {
    .question {
      color: #fff;
    }
  }

  .button-group {
    display: flex;
    gap: 12px;
    justify-content: center;
    margin-bottom: 16px;
  }

  .btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 24px;
    font-size: 15px;
    font-weight: 500;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif;
  }

  .btn-primary {
    background: #007aff;
    color: white;
  }

  .btn-primary:hover {
    background: #0051d5;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 122, 255, 0.4);
  }

  .btn-secondary {
    background: #f0f0f0;
    color: #333;
  }

  @media (prefers-color-scheme: dark) {
    .btn-secondary {
      background: #3a3a3a;
      color: #fff;
    }
  }

  .btn-secondary:hover {
    background: #e0e0e0;
    transform: translateY(-1px);
  }

  @media (prefers-color-scheme: dark) {
    .btn-secondary:hover {
      background: #4a4a4a;
    }
  }

  .hint {
    font-size: 13px;
    color: #999;
    margin: 0;
  }
</style>



