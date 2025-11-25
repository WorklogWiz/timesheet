<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { preferences } from '../lib/stores';
  import type { TaskSession, JiraIssue } from '../lib/types';
  import IssueSelector from '../components/IssueSelector.svelte';
  import { emit, listen } from '@tauri-apps/api/event';

  let sessions: TaskSession[] = [];
  let activeTimer: TaskSession | null = null;
  let loading = true;
  let error: string | null = null;
  let selectedIds = new Set<string>();
  let editingIssueKey: string | null = null;
  let showIssueSelector = false;
  let currentEditingSessionId: string | null = null;

  // Inline editing state
  let editingCell: { sessionId: string; field: string } | null = null;
  let editingValue = '';
  let editingDate = '';
  let editingTime = '';

  // Week grouping state
  let expandedWeeks = new Set<string>();

  interface WeekGroup {
    weekKey: string;
    weekLabel: string;
    sessions: TaskSession[];
    totalDuration: number;
  }

  // Add/Edit modal state
  let showEditModal = false;
  let editingSession: TaskSession | null = null;
  let editForm = {
    startDate: '',
    startTime: '',
    endDate: '',
    endTime: '',
    issueKey: '',
    comment: ''
  };

  // Delete confirmation modal state
  let showDeleteConfirmation = false;
  let sessionsToDelete: string[] = [];

  // Get time format preference from store
  let use24Hour = false;
  preferences.subscribe(prefs => {
    use24Hour = prefs.use24HourFormat;
  });

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

  onMount(async () => {
    await loadSessions();

    // Listen for refresh and session-updated events
    const unlistenRefresh = await listen('refresh-data', () => {
      loadSessions();
    });

    const unlistenUpdate = await listen('session-updated', () => {
      console.log('SessionsView received session-updated event');
      loadSessions(true); // Preserve expanded weeks when reloading from external updates
    });

    const unlistenTimerStarted = await listen('timer-started', () => {
      console.log('SessionsView received timer-started event');
      loadSessions(true); // Preserve expanded weeks when timer starts
    });

    return () => {
      unlistenRefresh();
      unlistenUpdate();
      unlistenTimerStarted();
    };
  });

  async function loadSessions(preserveExpandedWeeks = false) {
    try {
      loading = true;
      error = null;

      // Preserve the current expanded weeks state if requested
      const previousExpandedWeeks = preserveExpandedWeeks ? new Set(expandedWeeks) : null;

      sessions = await api.getAllSessions();
      activeTimer = await api.getActiveTimer();
      console.log('Loaded sessions:', sessions);
      console.log('Active timer:', activeTimer);

      // Restore expanded weeks or auto-expand only the most recent week
      if (previousExpandedWeeks && previousExpandedWeeks.size > 0) {
        expandedWeeks = previousExpandedWeeks;
      } else if (sessions.length > 0) {
        const weeks = groupSessionsByWeek(sessions);
        if (weeks.length > 0) {
          expandedWeeks = new Set([weeks[0].weekKey]);
        }
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      console.error('Error loading sessions:', e);
    } finally {
      loading = false;
    }
  }

  function getWeekStart(date: Date): Date {
    const d = new Date(date);
    const day = d.getDay();
    const diff = d.getDate() - day + (day === 0 ? -6 : 1); // Adjust when day is Sunday
    d.setDate(diff);
    d.setHours(0, 0, 0, 0);
    return d;
  }

  function formatWeekLabel(weekStart: Date): string {
    const weekEnd = new Date(weekStart);
    weekEnd.setDate(weekEnd.getDate() + 6);

    const startStr = weekStart.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
    const endStr = weekEnd.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });

    // Get week number
    const oneJan = new Date(weekStart.getFullYear(), 0, 1);
    const numberOfDays = Math.floor((weekStart.getTime() - oneJan.getTime()) / (24 * 60 * 60 * 1000));
    const weekNumber = Math.ceil((numberOfDays + oneJan.getDay() + 1) / 7);

    return `Week ${weekNumber} (${startStr} - ${endStr})`;
  }

  function groupSessionsByWeek(sessions: TaskSession[]): WeekGroup[] {
    const weeks = new Map<string, WeekGroup>();

    sessions.forEach(session => {
      const startDate = new Date(session.start_date);
      const weekStart = getWeekStart(startDate);
      const weekKey = weekStart.toISOString().split('T')[0];

      if (!weeks.has(weekKey)) {
        weeks.set(weekKey, {
          weekKey,
          weekLabel: formatWeekLabel(weekStart),
          sessions: [],
          totalDuration: 0
        });
      }

      const week = weeks.get(weekKey)!;
      week.sessions.push(session);

      // Calculate duration
      const start = new Date(session.start_date);
      const end = session.end_date ? new Date(session.end_date) : new Date();
      const duration = (end.getTime() - start.getTime()) / (1000 * 60 * 60); // hours
      week.totalDuration += duration;
    });

    // Sort weeks by date (most recent first)
    return Array.from(weeks.values()).sort((a, b) => b.weekKey.localeCompare(a.weekKey));
  }

  function toggleWeek(weekKey: string) {
    if (expandedWeeks.has(weekKey)) {
      expandedWeeks.delete(weekKey);
    } else {
      expandedWeeks.add(weekKey);
    }
    expandedWeeks = expandedWeeks; // Trigger reactivity
  }

  function formatHours(hours: number): string {
    const h = Math.floor(hours);
    const m = Math.round((hours - h) * 60);
    return `${h}h ${m}m`;
  }

  function startEditingCell(sessionId: string, field: string, currentValue: string) {
    editingCell = { sessionId, field };

    if (field === 'start_date' || field === 'end_date') {
      // Split datetime into date and time for editing
      const dt = new Date(currentValue);
      editingDate = dt.toISOString().split('T')[0]; // YYYY-MM-DD
      // Always use 24-hour format for editing: HH:MM
      const hours = dt.getHours().toString().padStart(2, '0');
      const minutes = dt.getMinutes().toString().padStart(2, '0');
      editingTime = `${hours}:${minutes}`;
      editingValue = '';
    } else {
      editingValue = currentValue;
      editingDate = '';
      editingTime = '';
    }
  }

  function cancelEditingCell() {
    editingCell = null;
    editingValue = '';
    editingDate = '';
    editingTime = '';
  }

  async function saveCell(sessionId: string, field: string) {
    try {
      const session = sessions.find(s => s.id === sessionId);
      if (!session) return;

      const updatedSession = { ...session };

      if (field === 'start_date' || field === 'end_date') {
        // Combine date and time
        const datetime = new Date(`${editingDate}T${editingTime}`);
        if (field === 'start_date') {
          updatedSession.start_date = datetime.toISOString();
        } else {
          updatedSession.end_date = datetime.toISOString();
        }
      } else if (field === 'comment') {
        updatedSession.comment = editingValue;
      }

      await api.updateSession(
        sessionId,
        updatedSession.start_date,
        updatedSession.end_date,
        updatedSession.issue_key || null,
        updatedSession.comment || null
      );

      // Emit event to refresh other views
      console.log('SessionsView (saveCell) emitting session-updated event for', sessionId);
      await emit('session-updated', { sessionId });

      await loadSessions(true);
      cancelEditingCell();

      // Scroll to the updated session
      setTimeout(() => scrollToSession(sessionId), 100);
    } catch (e) {
      console.error('Failed to update session:', e);
      alert('Failed to update session: ' + (e instanceof Error ? e.message : 'Unknown error'));
    }
  }

  function handleCellKeyDown(e: KeyboardEvent, sessionId: string, field: string) {
    if (e.key === 'Enter') {
      e.preventDefault();
      saveCell(sessionId, field);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      cancelEditingCell();
    }
  }

  function toggleSelection(id: string) {
    console.log('toggleSelection called for id:', id);
    if (selectedIds.has(id)) {
      selectedIds.delete(id);
      console.log('Deselected, new size:', selectedIds.size);
    } else {
      selectedIds.add(id);
      console.log('Selected, new size:', selectedIds.size);
    }
    selectedIds = selectedIds; // Trigger reactivity
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return '';
    const date = new Date(dateStr);
    // Use system locale (undefined) to respect user's regional settings
    return new Intl.DateTimeFormat(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      hour12: !use24Hour, // Use preference from store
    }).format(date);
  }

  function formatDuration(startDate: string, endDate: string | null): string {
    const start = new Date(startDate);
    const end = endDate ? new Date(endDate) : new Date();
    const diff = end.getTime() - start.getTime();
    const hours = Math.floor(diff / 3600000);
    const minutes = Math.floor((diff % 3600000) / 60000);
    const seconds = Math.floor((diff % 60000) / 1000);
    return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
  }

  function scrollToSession(sessionId: string) {
    const element = document.querySelector(`tr[data-session-id="${sessionId}"]`);
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'center' });
      // Add a brief highlight effect
      element.classList.add('highlight-flash');
      setTimeout(() => element.classList.remove('highlight-flash'), 1500);
    }
  }

  function handleDelete() {
    console.log('handleDelete called, selectedIds:', Array.from(selectedIds));

    if (selectedIds.size === 0) {
      console.log('No sessions selected, returning');
      return;
    }

    // Show custom confirmation modal
    sessionsToDelete = Array.from(selectedIds);
    showDeleteConfirmation = true;
  }

  async function confirmDelete() {
    console.log('Deleting sessions:', sessionsToDelete);
    showDeleteConfirmation = false;

    // Find the nearest session to focus on after delete
    let nearestSessionId: string | null = null;
    if (sessionsToDelete.length > 0) {
      const firstDeletedId = sessionsToDelete[0];
      const sessionIndex = sessions.findIndex(s => s.id === firstDeletedId);

      // Try to find the next session, or the previous one
      if (sessionIndex >= 0) {
        if (sessionIndex + 1 < sessions.length && !sessionsToDelete.includes(sessions[sessionIndex + 1].id)) {
          nearestSessionId = sessions[sessionIndex + 1].id;
        } else if (sessionIndex - 1 >= 0 && !sessionsToDelete.includes(sessions[sessionIndex - 1].id)) {
          nearestSessionId = sessions[sessionIndex - 1].id;
        }
      }
    }

    try {
      for (const id of sessionsToDelete) {
        console.log('Deleting session:', id);
        await api.deleteSession(id);
        console.log('Deleted session:', id);
      }

      // Emit event to refresh other views
      console.log('SessionsView (confirmDelete) emitting session-updated event');
      await emit('session-updated', { sessionId: null });

      selectedIds = new Set(); // Trigger reactivity
      sessionsToDelete = [];
      console.log('Reloading sessions...');
      await loadSessions(true);
      console.log('Sessions reloaded');

      // Focus on nearest session after reload
      if (nearestSessionId) {
        setTimeout(() => scrollToSession(nearestSessionId), 100);
      }
    } catch (e) {
      console.error('Delete error:', e);
      alert(`Error deleting sessions: ${e}`);
    }
  }

  function cancelDelete() {
    console.log('Delete cancelled');
    showDeleteConfirmation = false;
    sessionsToDelete = [];
  }

  function startEditingIssueKey(sessionId: string) {
    currentEditingSessionId = sessionId;
    showIssueSelector = true;
  }

  async function handleIssueSelect(issueKey: string | null) {
    if (!currentEditingSessionId) return;
    const sessionId = currentEditingSessionId;

    try {
      if (issueKey) {
        await api.assignIssueToSession(sessionId, issueKey);
      } else {
        // Remove issue assignment
        const session = sessions.find(s => s.id === sessionId);
        if (session) {
          await api.updateSession(
            sessionId,
            session.start_date,
            session.end_date,
            null,
            session.comment || null
          );
        }
      }

      showIssueSelector = false;
      currentEditingSessionId = null;

      // Emit event to refresh other views
      console.log('SessionsView (handleIssueSelect) emitting session-updated event for', sessionId);
      await emit('session-updated', { sessionId });

      await loadSessions(true);

      // Scroll to the updated session
      setTimeout(() => scrollToSession(sessionId), 100);
    } catch (e) {
      console.error('Failed to update issue key:', e);
      alert(`Error updating issue key: ${e}`);
    }
  }

  function handleIssueCancel() {
    showIssueSelector = false;
    currentEditingSessionId = null;
  }

  function openAddModal() {
    editingSession = null;
    const now = new Date();
    const today = now.toISOString().split('T')[0];
    const time = now.toTimeString().slice(0, 5);
    editForm = {
      startDate: today,
      startTime: time,
      endDate: today,
      endTime: time,
      issueKey: '',
      comment: ''
    };
    showEditModal = true;
  }

  function openEditModal() {
    if (selectedIds.size !== 1) return;
    const sessionId = Array.from(selectedIds)[0];
    const session = sessions.find(s => s.id === sessionId);
    if (!session) return;

    editingSession = session;
    const start = new Date(session.start_date);
    const end = session.end_date ? new Date(session.end_date) : new Date();

    editForm = {
      startDate: start.toISOString().split('T')[0],
      startTime: start.toTimeString().slice(0, 5),
      endDate: end.toISOString().split('T')[0],
      endTime: end.toTimeString().slice(0, 5),
      issueKey: session.issue_key || '',
      comment: session.comment || ''
    };
    showEditModal = true;
  }

  function closeEditModal() {
    showEditModal = false;
    editingSession = null;
  }

  async function saveSession() {
    try {
      const startDateTime = new Date(`${editForm.startDate}T${editForm.startTime}`).toISOString();
      const endDateTime = editForm.endDate && editForm.endTime
        ? new Date(`${editForm.endDate}T${editForm.endTime}`).toISOString()
        : null;

      if (editingSession) {
        // Update existing session
        await api.updateSession(
          editingSession.id,
          startDateTime,
          endDateTime,
          editForm.issueKey || null,
          editForm.comment || null
        );
      } else {
        // Add new session
        await api.addSession(
          startDateTime,
          endDateTime,
          editForm.issueKey || null,
          editForm.comment || null
        );
      }

      closeEditModal();
      await loadSessions();
    } catch (e) {
      alert(`Error saving session: ${e}`);
    }
  }
</script>

<div class="sessions-view">
  <div class="toolbar-container">
    <div class="drag-spacer" data-tauri-drag-region></div>
    <div class="button-group">
      <button class="btn btn-sm variant-filled icon-button" on:click={openAddModal} title="Add">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="12" y1="5" x2="12" y2="19"></line>
          <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
      </button>
      <button
        class="btn btn-sm variant-filled-error icon-button"
        disabled={selectedIds.size === 0}
        on:click={handleDelete}
        title="Delete"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="3 6 5 6 21 6"></polyline>
          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
        </svg>
      </button>
    </div>
  </div>

  <div class="content">
    {#if loading}
      <div class="loading">Loading sessions...</div>
    {:else if error}
      <div class="error">Error: {error}</div>
    {:else if sessions.length === 0}
      <div class="empty">No sessions yet. Start a timer to create your first session!</div>
    {:else}
      <div class="table-container">
        <table>
          <thead>
            <tr>
              <th class="w-10"></th>
              <th>Start Date</th>
              <th>Time Codes</th>
              <th class="w-12 text-center" title="Sync Status">Sync</th>
              <th>Comment</th>
              <th>End Date</th>
              <th class="w-24">Duration</th>
            </tr>
          </thead>
          <tbody>
            {#each groupSessionsByWeek(sessions) as week}
            <!-- Week header row -->
            <tr class="week-header" on:click={() => toggleWeek(week.weekKey)} role="button" tabindex="0" on:keydown={(e) => e.key === 'Enter' && toggleWeek(week.weekKey)}>
              <td colspan="7">
                <span class="week-toggle">{expandedWeeks.has(week.weekKey) ? '▼' : '▶'}</span>
                <span class="week-label">{week.weekLabel}</span>
                <span class="week-stats">{week.sessions.length} sessions · {formatHours(week.totalDuration)}</span>
              </td>
            </tr>
            <!-- Sessions for this week (only if expanded) -->
            {#if expandedWeeks.has(week.weekKey)}
              {#each week.sessions as session}
            <tr class:selected={selectedIds.has(session.id)} data-session-id={session.id}>
              <td>
                <input
                  type="checkbox"
                  checked={selectedIds.has(session.id)}
                  on:change={() => toggleSelection(session.id)}
                />
              </td>
              <td
                class="editable-cell"
                on:click={() => startEditingCell(session.id, 'start_date', session.start_date)}
                role="button"
                tabindex="0"
              >
                {#if editingCell?.sessionId === session.id && editingCell?.field === 'start_date'}
                  <div class="datetime-edit">
                    <input
                      type="date"
                      bind:value={editingDate}
                      on:blur={() => saveCell(session.id, 'start_date')}
                      on:keydown={(e) => handleCellKeyDown(e, session.id, 'start_date')}
                      class="inline-edit-input date-input"
                    />
                    <input
                      type="text"
                      bind:value={editingTime}
                      on:blur={() => saveCell(session.id, 'start_date')}
                      on:keydown={(e) => handleCellKeyDown(e, session.id, 'start_date')}
                      class="inline-edit-input time-input"
                      placeholder="HH:MM"
                      pattern="[0-2][0-9]:[0-5][0-9]"
                    />
                  </div>
                {:else}
                  {formatDate(session.start_date)}
                {/if}
              </td>
              <td>
                <div class="tags">
                  {#if session.issue_key}
                    {@const colors = getTagColor(session.issue_key)}
                    <span
                      class="tag issue-tag"
                      style="background: {colors.bg}; color: {colors.text}; box-shadow: inset 0 1px 2px {colors.shadow}, inset 0 0 1px {colors.shadow};"
                      on:click={() => startEditingIssueKey(session.id)}
                      on:keydown={(e) => e.key === 'Enter' && startEditingIssueKey(session.id)}
                      role="button"
                      tabindex="0"
                    >
                      {session.issue_key}
                    </span>
                  {:else}
                    <span class="tag no-issue-tag" on:click={() => startEditingIssueKey(session.id)} on:keydown={(e) => e.key === 'Enter' && startEditingIssueKey(session.id)} role="button" tabindex="0">
                      + Assign issue
                    </span>
                  {/if}
                  {#each session.time_codes as tc}
                    {@const colors = getTagColor(tc.name)}
                    <span
                      class="tag"
                      style="background: {colors.bg}; color: {colors.text}; box-shadow: inset 0 1px 2px {colors.shadow}, inset 0 0 1px {colors.shadow};"
                    >
                      {tc.name}
                    </span>
                  {/each}
                </div>
              </td>
              <td class="text-center sync-status-cell">
                {#if session.in_progress}
                  <span class="sync-icon in-progress" title="Timer in progress">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <circle cx="12" cy="12" r="10"/>
                      <polyline points="12 6 12 12 16 14"/>
                    </svg>
                  </span>
                {:else if session.synced_to_tracker}
                  <span class="sync-icon synced" title="Synced to issue tracker">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                      <polyline points="20 6 9 17 4 12"/>
                    </svg>
                  </span>
                {:else if session.issue_key}
                  <span class="sync-icon unsynced" title="Not synced - will sync on next sync operation">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.2"/>
                    </svg>
                  </span>
                {:else}
                  <span class="sync-icon no-issue" title="No issue assigned - cannot sync">—</span>
                {/if}
              </td>
              <td
                class="editable-cell"
                on:click={() => startEditingCell(session.id, 'comment', session.comment || '')}
                role="button"
                tabindex="0"
              >
                {#if editingCell?.sessionId === session.id && editingCell?.field === 'comment'}
                  <input
                    type="text"
                    bind:value={editingValue}
                    on:blur={() => saveCell(session.id, 'comment')}
                    on:keydown={(e) => handleCellKeyDown(e, session.id, 'comment')}
                    class="inline-edit-input"
                  />
                {:else}
                  {session.comment || ''}
                {/if}
              </td>
              <td
                class="editable-cell"
                on:click={() => startEditingCell(session.id, 'end_date', session.end_date || session.start_date)}
                role="button"
                tabindex="0"
              >
                {#if editingCell?.sessionId === session.id && editingCell?.field === 'end_date'}
                  <div class="datetime-edit">
                    <input
                      type="date"
                      bind:value={editingDate}
                      on:blur={() => saveCell(session.id, 'end_date')}
                      on:keydown={(e) => handleCellKeyDown(e, session.id, 'end_date')}
                      class="inline-edit-input date-input"
                    />
                    <input
                      type="text"
                      bind:value={editingTime}
                      on:blur={() => saveCell(session.id, 'end_date')}
                      on:keydown={(e) => handleCellKeyDown(e, session.id, 'end_date')}
                      class="inline-edit-input time-input"
                      placeholder="HH:MM"
                      pattern="[0-2][0-9]:[0-5][0-9]"
                    />
                  </div>
                {:else}
                  {formatDate(session.end_date)}
                {/if}
              </td>
              <td>{formatDuration(session.start_date, session.end_date)}</td>
            </tr>
              {/each}
            {/if}
          {/each}
        </tbody>
      </table>
      </div>
    {/if}
  </div>

  <div class="footer">
    <span>{sessions.length} entries</span>
    <span>{selectedIds.size} items selected</span>
  </div>

  <!-- Add/Edit Modal -->
  {#if showEditModal}
    <div
      class="modal-overlay"
      role="presentation"
      on:click={closeEditModal}
      on:keydown={(e) => e.key === 'Escape' && closeEditModal()}
    >
      <div
        class="modal"
        role="dialog"
        aria-modal="true"
        tabindex="-1"
        on:click|stopPropagation
        on:keydown|stopPropagation
      >
        <div class="modal-header">
          <h2>{editingSession ? 'Edit Session' : 'Add Session'}</h2>
          <button class="close-btn" on:click={closeEditModal}>✕</button>
        </div>

        <div class="modal-body">
          <div class="form-group">
            <label for="edit-start-date">Start Date & Time</label>
            <div class="datetime-input">
              <input id="edit-start-date" type="date" bind:value={editForm.startDate} />
              <input id="edit-start-time" type="time" bind:value={editForm.startTime} />
            </div>
          </div>

          <div class="form-group">
            <label for="edit-end-date">End Date & Time (optional)</label>
            <div class="datetime-input">
              <input id="edit-end-date" type="date" bind:value={editForm.endDate} />
              <input id="edit-end-time" type="time" bind:value={editForm.endTime} />
            </div>
          </div>

          <div class="form-group">
            <label for="edit-issue-key">Issue Key</label>
            <input id="edit-issue-key" type="text" bind:value={editForm.issueKey} placeholder="e.g., PROJ-123" />
          </div>

          <div class="form-group">
            <label for="edit-comment">Comment</label>
            <textarea id="edit-comment" bind:value={editForm.comment} placeholder="What did you work on?" rows="3"></textarea>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn variant-ghost-surface" on:click={closeEditModal}>Cancel</button>
          <button class="btn variant-filled-primary" on:click={saveSession}>Save</button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Delete Confirmation Modal -->
  {#if showDeleteConfirmation}
    <div
      class="modal-overlay"
      role="presentation"
      on:click={cancelDelete}
      on:keydown={(e) => e.key === 'Escape' && cancelDelete()}
    >
      <div
        class="modal confirmation-modal"
        role="dialog"
        aria-modal="true"
        tabindex="-1"
        on:click|stopPropagation
        on:keydown|stopPropagation
      >
        <div class="modal-header">
          <h2>Confirm Delete</h2>
          <button class="close-btn" on:click={cancelDelete}>✕</button>
        </div>

        <div class="modal-body">
          <p class="confirmation-text">
            Are you sure you want to delete {sessionsToDelete.length} session(s)?
          </p>
          <p class="confirmation-warning">
            This action cannot be undone.
          </p>
        </div>

        <div class="modal-footer">
          <button class="btn variant-ghost-surface" on:click={cancelDelete}>Cancel</button>
          <button class="btn variant-filled-error" on:click={confirmDelete}>Delete</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<!-- Issue Selector Component -->
<IssueSelector
  show={showIssueSelector}
  onSelect={handleIssueSelect}
  onCancel={handleIssueCancel}
/>

<style>
  .sessions-view {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
  }

  .toolbar-container {
    display: flex;
    align-items: center;
    height: 52px; /* macOS titlebar height */
    padding-left: 68px; /* Space for traffic lights */
    padding-right: 16px;
    border-bottom: 1px solid rgba(0, 0, 0, 0.1);
  }

  .button-group {
    display: flex;
    gap: 2px; /* Tight spacing like macOS */
    align-items: center;
  }

  .drag-spacer {
    flex: 1; /* Takes up all remaining space */
    height: 100%;
    min-width: 20px; /* Ensure there's always some draggable area */
  }

  /* Override Skeleton button styles for native macOS icon buttons */
  .toolbar-container :global(button.icon-button) {
    width: 32px !important;
    height: 32px !important;
    padding: 0 !important;
    display: inline-flex !important;
    align-items: center !important;
    justify-content: center !important;
    border-radius: 50% !important;
    border: none !important;
    transition: all 0.1s ease !important;
    min-width: unset !important;
  }

  .toolbar-container :global(button.icon-button svg) {
    width: 18px !important;
    height: 18px !important;
  }

  @media (prefers-color-scheme: light) {
    .toolbar-container {
      background-color: #ececec;
    }

    .toolbar-container :global(button.icon-button) {
      background: rgba(0, 0, 0, 0.04) !important;
      color: #000000 !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.15), inset 0 0 1px rgba(0, 0, 0, 0.1) !important;
    }

    .toolbar-container :global(button.icon-button:hover:not(:disabled)) {
      background: rgba(0, 0, 0, 0.08) !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.2), inset 0 0 1px rgba(0, 0, 0, 0.15) !important;
    }

    .toolbar-container :global(button.icon-button:active:not(:disabled)) {
      background: rgba(0, 0, 0, 0.12) !important;
      box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.25), inset 0 0 1px rgba(0, 0, 0, 0.2) !important;
    }

    .toolbar-container :global(button.icon-button:disabled) {
      opacity: 0.4 !important;
    }
  }

  @media (prefers-color-scheme: dark) {
    .toolbar-container {
      background-color: #323232;
      border-bottom-color: rgba(255, 255, 255, 0.1);
    }

    .toolbar-container :global(button.icon-button) {
      background: rgba(0, 0, 0, 0.25) !important;
      color: #ffffff !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.4), inset 0 0 1px rgba(0, 0, 0, 0.3) !important;
    }

    .toolbar-container :global(button.icon-button:hover:not(:disabled)) {
      background: rgba(0, 0, 0, 0.3) !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.5), inset 0 0 1px rgba(0, 0, 0, 0.4) !important;
    }

    .toolbar-container :global(button.icon-button:active:not(:disabled)) {
      background: rgba(0, 0, 0, 0.4) !important;
      box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.6), inset 0 0 1px rgba(0, 0, 0, 0.5) !important;
    }

    .toolbar-container :global(button.icon-button:disabled) {
      opacity: 0.4 !important;
    }
  }

  .content {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  .table-container {
    width: 100%;
    height: 100%;
  }

  .loading, .error, .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 48px 24px;
    color: var(--text-secondary);
  }

  .error {
    color: #ff3b30;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    background: var(--bg-primary);
    table-layout: fixed; /* Prevent resizing when editing */
  }

  /* Fixed column widths to prevent jumping */
  th:nth-child(1), td:nth-child(1) { width: 40px; } /* Checkbox */
  th:nth-child(2), td:nth-child(2) { width: 200px; } /* Start Date */
  th:nth-child(3), td:nth-child(3) { width: 180px; } /* Time Codes */
  th:nth-child(4), td:nth-child(4) { width: auto; } /* Comment - flexible */
  th:nth-child(5), td:nth-child(5) { width: 200px; } /* End Date */
  th:nth-child(6), td:nth-child(6) { width: 100px; } /* Duration */

  .editable-cell {
    cursor: pointer;
    position: relative;
    min-width: 120px; /* Prevent collapsing */
  }

  .editable-cell:hover {
    background: rgba(0, 122, 255, 0.05);
  }

  .datetime-edit {
    display: flex;
    gap: 4px;
    width: 100%;
  }

  .inline-edit-input {
    padding: 4px 8px;
    margin: 0;
    margin-left: -8px; /* Align with text position */
    border: none;
    border-radius: 4px;
    font-size: 13px;
    font-family: inherit;
    background: rgba(0, 122, 255, 0.08);
    color: var(--text-primary);
    line-height: 1.4;
    box-sizing: border-box;
  }

  .date-input {
    flex: 1;
    min-width: 120px;
  }

  .time-input {
    width: 60px;
    flex-shrink: 0;
    font-family: 'SF Mono', Monaco, 'Courier New', monospace;
    text-align: left;
  }

  .inline-edit-input:focus {
    outline: none;
    background: rgba(0, 122, 255, 0.12);
    box-shadow: inset 0 0 0 2px #007aff;
  }

  /* Style time/date picker indicators */
  .inline-edit-input::-webkit-calendar-picker-indicator {
    opacity: 0.6;
    cursor: pointer;
  }

  .inline-edit-input::-webkit-calendar-picker-indicator:hover {
    opacity: 1;
  }

  td {
    padding: 8px 12px;
    vertical-align: middle;
  }

  .footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-color);
    font-size: 13px;
    color: var(--text-secondary);
  }

  thead {
    position: sticky;
    top: 0;
    background: var(--bg-secondary);
    z-index: 1;
  }

  th {
    text-align: left;
    padding: 8px 16px;
    font-weight: 600;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .week-header {
    background: var(--bg-secondary);
    cursor: pointer;
    user-select: none;
    transition: background 0.1s;
  }

  .week-header:hover {
    background: rgba(0, 122, 255, 0.05);
  }

  .week-header td {
    padding: 12px 16px !important;
    font-weight: 600;
    border-bottom: 1px solid var(--border-color);
  }

  .week-toggle {
    display: inline-block;
    width: 20px;
    font-size: 10px;
    color: var(--text-secondary);
  }

  .week-label {
    font-size: 14px;
    color: var(--text-primary);
    margin-right: 12px;
  }

  .week-stats {
    font-size: 12px;
    color: var(--text-secondary);
    font-weight: 400;
  }

  td {
    padding: 10px 16px;
    font-size: 13px;
    border-bottom: 1px solid var(--border-color);
  }

  tbody tr:hover {
    background: var(--bg-secondary);
  }

  tr.selected {
    background: rgba(0, 122, 255, 0.08);
  }

  input[type="checkbox"] {
    cursor: pointer;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }

  .tag {
    display: inline-flex;
    align-items: center;
    padding: 4px 12px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 500;
    border: none;
    transition: all 0.1s ease;
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.4;
  }

  /* Issue tag (sunken, elliptical) */
  .issue-tag {
    cursor: pointer;
  }

  /* No issue tag (clickable) */
  .no-issue-tag {
    cursor: pointer;
    font-size: 11px;
    padding: 4px 10px;
  }

  @media (prefers-color-scheme: light) {
    .no-issue-tag {
      background: rgba(0, 0, 0, 0.04);
      color: #666666;
      box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.12), inset 0 0 1px rgba(0, 0, 0, 0.08);
    }

    .no-issue-tag:hover {
      background: rgba(0, 0, 0, 0.08);
    }
  }

  @media (prefers-color-scheme: dark) {
    .no-issue-tag {
      background: rgba(255, 255, 255, 0.08);
      color: #999999;
      box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.3), inset 0 0 1px rgba(0, 0, 0, 0.2);
    }

    .no-issue-tag:hover {
      background: rgba(255, 255, 255, 0.12);
    }
  }

  .footer {
    display: flex;
    justify-content: space-between;
    padding: 12px 16px;
    border-top: 1px solid var(--border-color);
    background: var(--bg-secondary);
    font-size: 12px;
    color: var(--text-secondary);
  }

  /* Modal styles */
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-primary);
    border-radius: 12px;
    width: 90%;
    max-width: 500px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border-color);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .close-btn {
    padding: 4px 8px;
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 20px;
    color: var(--text-secondary);
    transition: color 0.15s;
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .modal-body {
    padding: 24px;
  }

  .form-group {
    margin-bottom: 20px;
  }

  .form-group:last-child {
    margin-bottom: 0;
  }

  .form-group label {
    display: block;
    margin-bottom: 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .form-group input[type="text"],
  .form-group input[type="date"],
  .form-group input[type="time"],
  .form-group textarea {
    width: 100%;
    padding: 8px 12px;
    font-size: 14px;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .form-group textarea {
    resize: vertical;
    font-family: inherit;
  }

  .datetime-input {
    display: grid;
    grid-template-columns: 1fr 120px;
    gap: 8px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 24px;
    border-top: 1px solid rgb(var(--color-surface-300));
  }

  /* Confirmation modal */
  .confirmation-modal {
    max-width: 400px;
  }

  .confirmation-text {
    margin: 0 0 12px 0;
    font-size: 15px;
    color: var(--text-primary);
  }

  .confirmation-warning {
    margin: 0;
    font-size: 13px;
    color: var(--text-secondary);
    font-style: italic;
  }

  /* Highlight flash effect for focused row */
  :global(tr.highlight-flash) {
    animation: flash 1.5s ease-in-out;
  }

  @keyframes flash {
    0%, 100% { background-color: transparent; }
    50% { background-color: rgba(0, 122, 255, 0.2); }
  }

  /* Sync status */
  .sync-status-cell {
    padding: 8px;
    vertical-align: middle;
  }

  .sync-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .sync-icon svg {
    display: block;
  }

  .sync-icon.synced {
    color: #666;
    opacity: 0.8;
  }

  .sync-icon.unsynced {
    color: #888;
    opacity: 0.7;
    animation: pulse 2s ease-in-out infinite;
  }

  .sync-icon.in-progress {
    color: #555;
    opacity: 0.7;
  }

  .sync-icon.no-issue {
    opacity: 0.3;
    font-size: 14px;
    color: #999;
  }

  /* Conflict state - to be implemented when conflict detection is added */
  /* .sync-icon.conflict {
    color: #d32f2f;
    opacity: 1;
    cursor: pointer;
  }

  .sync-icon.conflict:hover {
    opacity: 0.8;
  } */

  @keyframes pulse {
    0%, 100% { opacity: 0.7; }
    50% { opacity: 0.4; }
  }
</style>

