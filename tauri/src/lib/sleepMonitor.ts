import { listen, emit } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import * as api from './api';

export async function initSleepMonitor() {
  console.log('🛌 Initializing sleep monitor...');

  // Request notification permissions
  let permissionGranted = await isPermissionGranted();
  if (!permissionGranted) {
    console.log('📱 Requesting notification permission...');
    const permission = await requestPermission();
    permissionGranted = permission === 'granted';
  }

  if (!permissionGranted) {
    console.warn('⚠️ Notification permission not granted - sleep alerts will not be shown');
    return;
  }

  console.log('✅ Notification permission granted');

  // Listen for system wake events
  await listen('system-wake', async (event) => {
    console.log('⏰ System wake event received:', event.payload);

    const payload = event.payload as { sleep_duration_seconds: number };
    const sleepDurationSeconds = payload.sleep_duration_seconds;

    // Check if there's an active timer
    const activeTimer = await api.getActiveTimer();

    if (!activeTimer) {
      console.log('ℹ️ No active timer running - skipping sleep dialog');
      return;
    }

    console.log('🏃 Active timer found:', activeTimer.issue_key || 'no issue');

    // Format sleep duration
    const hours = Math.floor(sleepDurationSeconds / 3600);
    const minutes = Math.floor((sleepDurationSeconds % 3600) / 60);
    const durationText = hours > 0
      ? `${hours}h ${minutes}m`
      : `${minutes}m`;

    console.log(`💤 System was asleep for ${durationText} - showing notification and dialog`);

    // Show native notification
    await sendNotification({
      title: 'System Wake Detected',
      body: `Your computer was asleep for ${durationText}.\nYou have an active timer running${activeTimer.issue_key ? ` for ${activeTimer.issue_key}` : ''}.\n\nWhat would you like to do?`,
    });

    // Since macOS notifications don't support action buttons via Tauri,
    // we'll show a modal dialog in the app instead
    showSleepDialog(durationText, activeTimer.issue_key || null);
  });

  console.log('✅ Sleep monitor initialized and listening for system-wake events');
}

function showSleepDialog(sleepDuration: string, issueKey: string | null) {
  // Emit event to show dialog in the UI
  emit('show-sleep-dialog', {
    sleepDuration,
    issueKey,
  });
}

export async function handleSleepResponse(action: 'stop' | 'continue') {
  console.log(`🎯 User selected: ${action}`);

  if (action === 'stop') {
    console.log('⏹️ Stopping timer due to sleep...');
    await invoke('handle_sleep_stop');
    console.log('✅ Timer stopped');
  } else if (action === 'continue') {
    console.log('▶️ Continuing timer...');
    await invoke('handle_sleep_continue');
    console.log('✅ Timer continues running');
  }
}

