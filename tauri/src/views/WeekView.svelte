<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { api } from '../lib/api';
  import { preferences } from '../lib/stores';
  import type { TaskSession } from '../lib/types';
  import IssueSelector from '../components/IssueSelector.svelte';
  import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';

  let sessions: TaskSession[] = [];
  let activeTimer: TaskSession | null = null;
  let currentWeekStart = getWeekStart(new Date());
  let loading = true;
  let currentTime = new Date();
  const hours = Array.from({ length: 24 }, (_, i) => i);
  let savedScrollPosition = 0; // Store scroll position when navigating weeks

  // Inline editing state
  let editingSessionId: string | null = null;
  let editingField: string | null = null;
  let editingValue = '';
  let editingStartTime = '';
  let editingEndTime = '';
  let showIssueSelector = false;
  let currentEditingSession: TaskSession | null = null;

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

  function getWeekStart(date: Date): Date {
    const d = new Date(date);
    const day = d.getDay(); // 0 = Sunday, 1 = Monday, etc.
    // Calculate days to Monday (first day of week)
    // If Sunday (0), go back 6 days; otherwise go back (day - 1) days
    const diff = day === 0 ? -6 : 1 - day;
    d.setDate(d.getDate() + diff);
    // Set to midnight (start of day)
    d.setHours(0, 0, 0, 0);
    return d;
  }

  function formatDate(date: Date): string {
    // Use system locale (undefined) to respect user's regional settings
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  }

  function formatTime(hour: number): string {
    // Create a date with the given hour and use preference for 12/24 hour format
    const date = new Date();
    date.setHours(hour, 0, 0, 0);
    return new Intl.DateTimeFormat(undefined, {
      hour: 'numeric',
      hour12: !use24Hour, // Use preference from store
    }).format(date);
  }

  function previousWeek() {
    // Save current scroll position
    const container = document.querySelector('.calendar-container');
    if (container) {
      savedScrollPosition = container.scrollTop;
    }
    currentWeekStart = new Date(currentWeekStart.setDate(currentWeekStart.getDate() - 7));
    loadSessions();
    // Restore scroll position after a brief delay
    setTimeout(() => restoreScrollPosition(), 50);
  }

  function nextWeek() {
    // Save current scroll position
    const container = document.querySelector('.calendar-container');
    if (container) {
      savedScrollPosition = container.scrollTop;
    }
    currentWeekStart = new Date(currentWeekStart.setDate(currentWeekStart.getDate() + 7));
    loadSessions();
    // Restore scroll position after a brief delay
    setTimeout(() => restoreScrollPosition(), 50);
  }

  function goToToday() {
    currentWeekStart = getWeekStart(new Date());
    loadSessions();
    // Reset to default scroll when going to today
    setTimeout(() => scrollToWorkingHours(), 50);
  }

  function restoreScrollPosition() {
    const container = document.querySelector('.calendar-container');
    if (container && savedScrollPosition > 0) {
      container.scrollTop = savedScrollPosition;
    }
  }

  function scrollToWorkingHours() {
    const container = document.querySelector('.calendar-container');
    if (container) {
      // Default to 7:00 AM (working hours start)
      // Each hour is 60px, so 7 AM is at 7 * 60 = 420px
      container.scrollTop = 7 * 60;
    }
  }

  function getWeekTitle(): string {
    // Get the month name
    const month = currentWeekStart.toLocaleDateString(undefined, { month: 'long', year: 'numeric' });

    // Calculate week number (ISO 8601)
    const oneJan = new Date(currentWeekStart.getFullYear(), 0, 1);
    const numberOfDays = Math.floor((currentWeekStart.getTime() - oneJan.getTime()) / (24 * 60 * 60 * 1000));
    const weekNumber = Math.ceil((numberOfDays + oneJan.getDay() + 1) / 7);

    return `${month} - Week ${weekNumber}`;
  }

  function getWeekTotalHours(): string {
    let totalMinutes = 0;

    for (const session of sessions) {
      const start = new Date(session.start_date);
      const end = session.end_date ? new Date(session.end_date) : new Date();
      const durationMinutes = (end.getTime() - start.getTime()) / (1000 * 60);
      totalMinutes += durationMinutes;
    }

    const hours = Math.floor(totalMinutes / 60);
    const minutes = Math.round(totalMinutes % 60);

    if (hours === 0 && minutes === 0) {
      return '';
    }

    if (minutes === 0) {
      return `${hours}h`;
    }

    return `${hours}h ${minutes}m`;
  }

  function startEditingTime(sessionId: string, session: TaskSession) {
    editingSessionId = sessionId;
    editingField = 'time';

    const start = new Date(session.start_date);
    const end = session.end_date ? new Date(session.end_date) : new Date();

    const startHours = start.getHours().toString().padStart(2, '0');
    const startMinutes = start.getMinutes().toString().padStart(2, '0');
    const endHours = end.getHours().toString().padStart(2, '0');
    const endMinutes = end.getMinutes().toString().padStart(2, '0');

    editingStartTime = `${startHours}:${startMinutes}`;
    editingEndTime = `${endHours}:${endMinutes}`;
  }

  function startEditingComment(sessionId: string, currentComment: string) {
    editingSessionId = sessionId;
    editingField = 'comment';
    editingValue = currentComment || '';
  }

  function startEditingIssueKey(session: TaskSession) {
    currentEditingSession = session;
    showIssueSelector = true;
  }

  function cancelEditing() {
    editingSessionId = null;
    editingField = null;
    editingValue = '';
    editingStartTime = '';
    editingEndTime = '';
    showIssueSelector = false;
    currentEditingSession = null;
  }

  async function handleIssueSelect(issueKey: string | null) {
    if (!currentEditingSession) return;

    try {
      await api.updateSession(
        currentEditingSession.id,
        currentEditingSession.start_date,
        currentEditingSession.end_date,
        issueKey,
        currentEditingSession.comment || null
      );

      // Emit event to refresh other views
      console.log('WeekView (handleIssueSelect) emitting session-updated event for', currentEditingSession.id);
      await emit('session-updated', { sessionId: currentEditingSession.id });

      showIssueSelector = false;
      currentEditingSession = null;
      await loadSessions();
    } catch (e) {
      console.error('Failed to update issue key:', e);
      alert(`Error updating issue key: ${e}`);
    }
  }

  function handleIssueCancel() {
    showIssueSelector = false;
    currentEditingSession = null;
  }

  async function saveTime(sessionId: string, session: TaskSession) {
    try {
      // Get the date from the original session
      const sessionDate = new Date(session.start_date);
      const dateStr = sessionDate.toISOString().split('T')[0]; // YYYY-MM-DD

      // Combine date with new times
      const startDateTime = new Date(`${dateStr}T${editingStartTime}`).toISOString();
      const endDateTime = editingEndTime
        ? new Date(`${dateStr}T${editingEndTime}`).toISOString()
        : null;

      await api.updateSession(
        sessionId,
        startDateTime,
        endDateTime,
        session.issue_key || null,
        session.comment || null
      );

      // Emit event to refresh other views
      console.log('WeekView (saveTime) emitting session-updated event for', sessionId);
      await emit('session-updated', { sessionId });

      cancelEditing();
      await loadSessions();
    } catch (e) {
      console.error('Failed to update time:', e);
      alert(`Error updating time: ${e}`);
    }
  }

  async function saveComment(sessionId: string, session: TaskSession) {
    try {
      await api.updateSession(
        sessionId,
        session.start_date,
        session.end_date,
        session.issue_key || null,
        editingValue || null
      );

      // Emit event to refresh other views
      console.log('WeekView (saveComment) emitting session-updated event for', sessionId);
      await emit('session-updated', { sessionId });

      cancelEditing();
      await loadSessions();
    } catch (e) {
      console.error('Failed to update comment:', e);
      alert(`Error updating comment: ${e}`);
    }
  }

  function handleKeyDown(e: KeyboardEvent, sessionId: string, field: string, session: TaskSession) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (field === 'time') {
        saveTime(sessionId, session);
      } else if (field === 'comment') {
        saveComment(sessionId, session);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      cancelEditing();
    }
  }

  function isToday(date: Date): boolean {
    const today = new Date();
    return date.toDateString() === today.toDateString();
  }

  function getSessionPosition(session: TaskSession): { top: string; height: string } {
    const start = new Date(session.start_date);
    const end = session.end_date ? new Date(session.end_date) : new Date();

    const startMinutes = start.getHours() * 60 + start.getMinutes();
    const endMinutes = end.getHours() * 60 + end.getMinutes();
    const duration = endMinutes - startMinutes;

    // Each hour is 60px
    const top = (startMinutes / 60) * 60;
    const height = Math.max((duration / 60) * 60, 30); // Minimum 30px height

    return {
      top: `${top}px`,
      height: `${height}px`
    };
  }

  function getCurrentTimePosition(): string {
    const now = new Date();
    const minutes = now.getHours() * 60 + now.getMinutes();
    return `${(minutes / 60) * 60}px`;
  }

  function getTotalHoursForDay(daySessions: TaskSession[]): string {
    let totalMinutes = 0;

    for (const session of daySessions) {
      const start = new Date(session.start_date);
      const end = session.end_date ? new Date(session.end_date) : new Date();
      const durationMinutes = (end.getTime() - start.getTime()) / (1000 * 60);
      totalMinutes += durationMinutes;
    }

    const hours = Math.floor(totalMinutes / 60);
    const minutes = Math.round(totalMinutes % 60);

    if (hours === 0 && minutes === 0) {
      return '';
    }

    if (minutes === 0) {
      return `${hours}h`;
    }

    return `${hours}h ${minutes}m`;
  }

  async function loadSessions() {
    loading = true;
    try {
      const allSessions = await api.getAllSessions();
      activeTimer = await api.getActiveTimer();
      console.log('WeekView - Active timer:', activeTimer);

      const weekEnd = new Date(currentWeekStart);
      weekEnd.setDate(weekEnd.getDate() + 7);

      sessions = allSessions.filter(s => {
        const startDate = new Date(s.start_date);
        return startDate >= currentWeekStart && startDate < weekEnd;
      });
    } catch (error) {
      console.error('Failed to load sessions:', error);
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    loadSessions();

    // Listen for refresh and session-updated events
    const unlistenRefresh = await listen('refresh-data', () => {
      loadSessions();
    });

    const unlistenUpdate = await listen('session-updated', () => {
      console.log('WeekView received session-updated event');
      loadSessions();
    });

    const unlistenTimerStarted = await listen('timer-started', () => {
      console.log('WeekView received timer-started event');
      loadSessions();
    });

    // Scroll to working hours (7:00 AM - 7:00 PM)
    setTimeout(() => {
      scrollToWorkingHours();
    }, 100); // Small delay to ensure DOM is ready

    // Update current time every minute
    const interval = setInterval(() => {
      currentTime = new Date();
    }, 60000);

    return () => {
      clearInterval(interval);
      unlistenRefresh();
      unlistenUpdate();
      unlistenTimerStarted();
    };
  });
</script>

<div class="week-view">
  <div class="week-navigation-bar">
    <div class="week-title">
      <span class="week-title-text">{getWeekTitle()}</span>
      {#if getWeekTotalHours()}
        <span class="week-total-hours">({getWeekTotalHours()})</span>
      {/if}
    </div>
    <div class="drag-spacer" data-tauri-drag-region></div>
    <button class="btn btn-sm variant-filled today-button" on:click={goToToday}>Today</button>
    <div class="button-group">
      <button class="btn btn-sm variant-filled icon-button" on:click={previousWeek} title="Previous Week">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="15 18 9 12 15 6"></polyline>
        </svg>
      </button>
      <button class="btn btn-sm variant-filled icon-button" on:click={nextWeek} title="Next Week">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="9 18 15 12 9 6"></polyline>
        </svg>
      </button>
    </div>
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else}
    <div class="calendar-container">
      <div class="calendar-grid">
        <!-- Time labels column -->
        <div class="time-column">
          <div class="time-header"></div>
          <div class="time-labels">
            {#each hours as hour}
              <div class="time-label">
                {formatTime(hour)}
              </div>
            {/each}
          </div>
          <div class="time-footer"></div>
        </div>

        <!-- Day columns -->
        {#each Array(7) as _, dayIndex}
          {@const dayDate = new Date(currentWeekStart.getTime() + dayIndex * 24 * 60 * 60 * 1000)}
          {@const daySessions = sessions.filter(s => {
            const sessionDate = new Date(s.start_date);
            return sessionDate.toDateString() === dayDate.toDateString();
          })}

          <div class="day-column">
            <div class="day-header" class:today={isToday(dayDate)}>
              <div class="day-name">{dayDate.toLocaleDateString(undefined, { weekday: 'short' })}</div>
              <div class="day-date" class:today-date={isToday(dayDate)}>{dayDate.getDate()}</div>
            </div>

            <div class="day-content">
              <!-- Hour grid lines -->
              <div class="hour-lines">
                {#each hours as hour}
                  <div class="hour-line"></div>
                {/each}
              </div>

              <!-- Current time indicator -->
              {#if isToday(dayDate)}
                <div class="current-time-line" style="top: {getCurrentTimePosition()}"></div>
              {/if}

              <!-- Sessions -->
              <div class="sessions-container">
                {#each daySessions as session}
                  {@const pos = getSessionPosition(session)}
                  <div
                    class="session-block"
                    class:in-progress={session.in_progress}
                    class:editing={editingSessionId === session.id}
                    style="top: {pos.top}; height: {pos.height}"
                  >
                    <div class="session-content">
                      <div class="session-tags">
                        {#if session.issue_key}
                          {@const colors = getTagColor(session.issue_key)}
                          <span
                            class="session-tag editable"
                            style="background: {colors.bg}; color: {colors.text}; box-shadow: inset 0 1px 2px {colors.shadow}, inset 0 0 1px {colors.shadow}; cursor: pointer;"
                            on:click={() => startEditingIssueKey(session)}
                            on:keydown={(e) => e.key === 'Enter' && startEditingIssueKey(session)}
                            role="button"
                            tabindex="0"
                          >
                            {session.issue_key}
                          </span>
                        {:else}
                          <span
                            class="session-tag no-issue-tag"
                            on:click={() => startEditingIssueKey(session)}
                            on:keydown={(e) => e.key === 'Enter' && startEditingIssueKey(session)}
                            role="button"
                            tabindex="0"
                          >
                            + Issue
                          </span>
                        {/if}
                        {#each session.time_codes as tc}
                          {@const colors = getTagColor(tc.name)}
                          <span
                            class="session-tag"
                            style="background: {colors.bg}; color: {colors.text}; box-shadow: inset 0 1px 2px {colors.shadow}, inset 0 0 1px {colors.shadow};"
                          >
                            {tc.name}
                          </span>
                        {/each}
                      </div>

                      <!-- Time editing -->
                      {#if editingSessionId === session.id && editingField === 'time'}
                        <div class="session-time-edit">
                          <input
                            type="text"
                            bind:value={editingStartTime}
                            on:blur={() => saveTime(session.id, session)}
                            on:keydown={(e) => handleKeyDown(e, session.id, 'time', session)}
                            class="time-input-inline"
                            placeholder="HH:MM"
                            pattern="[0-2][0-9]:[0-5][0-9]"
                          />
                          <span>-</span>
                          <input
                            type="text"
                            bind:value={editingEndTime}
                            on:blur={() => saveTime(session.id, session)}
                            on:keydown={(e) => handleKeyDown(e, session.id, 'time', session)}
                            class="time-input-inline"
                            placeholder="HH:MM"
                            pattern="[0-2][0-9]:[0-5][0-9]"
                          />
                        </div>
                      {:else}
                        <div
                          class="session-time editable"
                          on:click={() => startEditingTime(session.id, session)}
                          on:keydown={(e) => e.key === 'Enter' && startEditingTime(session.id, session)}
                          role="button"
                          tabindex="0"
                        >
                          {new Date(session.start_date).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit', hour12: !use24Hour })}
                          {#if session.end_date}
                            - {new Date(session.end_date).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit', hour12: !use24Hour })}
                          {/if}
                        </div>
                      {/if}

                      <!-- Comment editing -->
                      {#if editingSessionId === session.id && editingField === 'comment'}
                        <input
                          type="text"
                          bind:value={editingValue}
                          on:blur={() => saveComment(session.id, session)}
                          on:keydown={(e) => handleKeyDown(e, session.id, 'comment', session)}
                          class="comment-input-inline"
                          placeholder="Add comment..."
                        />
                      {:else}
                        <div
                          class="session-comment editable"
                          on:click={() => startEditingComment(session.id, session.comment || '')}
                          on:keydown={(e) => e.key === 'Enter' && startEditingComment(session.id, session.comment || '')}
                          role="button"
                          tabindex="0"
                        >
                          {session.comment || 'Add comment...'}
                        </div>
                      {/if}

                      <!-- Sync status badge -->
                      <div class="sync-status-badge">
                        {#if session.in_progress}
                          <span class="sync-icon in-progress" title="Timer in progress">
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                              <circle cx="12" cy="12" r="10"/>
                              <polyline points="12 6 12 12 16 14"/>
                            </svg>
                          </span>
                        {:else if session.synced_to_tracker}
                          <span class="sync-icon synced" title="Synced to issue tracker">
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                              <polyline points="20 6 9 17 4 12"/>
                            </svg>
                          </span>
                        {:else if session.issue_key}
                          <span class="sync-icon unsynced" title="Not synced">
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                              <path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.2"/>
                            </svg>
                          </span>
                        {/if}
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            </div>

            <!-- Day total footer -->
            <div class="day-footer">
              <span class="day-total">{getTotalHoursForDay(daySessions) || '0h'}</span>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Issue Selector Component -->
  <IssueSelector
    show={showIssueSelector}
    onSelect={handleIssueSelect}
    onCancel={handleIssueCancel}
  />
</div>

<style>
  .week-view {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
  }

  .week-navigation-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    height: 52px; /* macOS titlebar height */
    padding-left: 68px; /* Space for traffic lights */
    padding-right: 20px;
    border-bottom: 1px solid rgba(0, 0, 0, 0.1);
  }

  .week-title {
    position: absolute;
    left: 100px; /* Position just after traffic lights */
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    user-select: none;
    pointer-events: none; /* Allow dragging through the title */
  }

  .week-title-text {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .week-total-hours {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .drag-spacer {
    flex: 1;
    height: 100%;
  }

  .button-group {
    display: flex;
    gap: 2px;
    align-items: center;
  }

  /* Today button styling */
  .week-navigation-bar :global(button.today-button) {
    height: 28px !important;
    padding: 0 16px !important;
    font-size: 13px !important;
    font-weight: 500 !important;
    border-radius: 14px !important;
    border: none !important;
    transition: all 0.1s ease !important;
    margin-right: 12px !important;
  }

  /* Icon button styling (arrows) */
  .week-navigation-bar :global(button.icon-button) {
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

  .week-navigation-bar :global(button.icon-button svg) {
    width: 18px !important;
    height: 18px !important;
  }

  @media (prefers-color-scheme: light) {
    .week-navigation-bar {
      background-color: #ececec;
    }

    /* Today button */
    .week-navigation-bar :global(button.today-button) {
      background: rgba(0, 0, 0, 0.04) !important;
      color: #000000 !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.15), inset 0 0 1px rgba(0, 0, 0, 0.1) !important;
    }

    .week-navigation-bar :global(button.today-button:hover) {
      background: rgba(0, 0, 0, 0.08) !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.2), inset 0 0 1px rgba(0, 0, 0, 0.15) !important;
    }

    .week-navigation-bar :global(button.today-button:active) {
      background: rgba(0, 0, 0, 0.12) !important;
      box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.25), inset 0 0 1px rgba(0, 0, 0, 0.2) !important;
    }

    /* Icon buttons */
    .week-navigation-bar :global(button.icon-button) {
      background: rgba(0, 0, 0, 0.04) !important;
      color: #000000 !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.15), inset 0 0 1px rgba(0, 0, 0, 0.1) !important;
    }

    .week-navigation-bar :global(button.icon-button:hover) {
      background: rgba(0, 0, 0, 0.08) !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.2), inset 0 0 1px rgba(0, 0, 0, 0.15) !important;
    }

    .week-navigation-bar :global(button.icon-button:active) {
      background: rgba(0, 0, 0, 0.12) !important;
      box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.25), inset 0 0 1px rgba(0, 0, 0, 0.2) !important;
    }
  }

  @media (prefers-color-scheme: dark) {
    .week-navigation-bar {
      background-color: #323232;
      border-bottom-color: rgba(255, 255, 255, 0.1);
    }

    /* Today button */
    .week-navigation-bar :global(button.today-button) {
      background: rgba(0, 0, 0, 0.25) !important;
      color: #ffffff !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.4), inset 0 0 1px rgba(0, 0, 0, 0.3) !important;
    }

    .week-navigation-bar :global(button.today-button:hover) {
      background: rgba(0, 0, 0, 0.3) !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.5), inset 0 0 1px rgba(0, 0, 0, 0.4) !important;
    }

    .week-navigation-bar :global(button.today-button:active) {
      background: rgba(0, 0, 0, 0.4) !important;
      box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.6), inset 0 0 1px rgba(0, 0, 0, 0.5) !important;
    }

    /* Icon buttons */
    .week-navigation-bar :global(button.icon-button) {
      background: rgba(0, 0, 0, 0.25) !important;
      color: #ffffff !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.4), inset 0 0 1px rgba(0, 0, 0, 0.3) !important;
    }

    .week-navigation-bar :global(button.icon-button:hover) {
      background: rgba(0, 0, 0, 0.3) !important;
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.5), inset 0 0 1px rgba(0, 0, 0, 0.4) !important;
    }

    .week-navigation-bar :global(button.icon-button:active) {
      background: rgba(0, 0, 0, 0.4) !important;
      box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.6), inset 0 0 1px rgba(0, 0, 0, 0.5) !important;
    }
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
  }

  .calendar-container {
    flex: 1;
    overflow: auto;
    background: var(--bg-primary);
  }

  .calendar-grid {
    display: grid;
    grid-template-columns: 60px repeat(7, 1fr);
    min-height: 100%;
  }

  /* Time column */
  .time-column {
    position: sticky;
    left: 0;
    z-index: 10;
    background: var(--bg-primary);
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
  }

  .time-header {
    height: 60px;
    border-bottom: 1px solid var(--border-color);
    position: sticky;
    top: 0;
    z-index: 10;
    background: var(--bg-primary);
  }

  .time-labels {
    position: relative;
    flex: 1;
  }

  .time-footer {
    position: sticky;
    bottom: 0;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-color);
    min-height: 40px;
    z-index: 10;
  }

  .time-label {
    height: 60px;
    display: flex;
    align-items: flex-start;
    justify-content: flex-end;
    padding: 0 8px;
    font-size: 11px;
    color: var(--text-secondary);
    transform: translateY(-8px);
  }

  /* Day columns */
  .day-column {
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
  }

  .day-column:last-child {
    border-right: none;
  }

  .day-footer {
    position: sticky;
    bottom: 0;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-color);
    padding: 8px;
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 5;
    min-height: 40px;
  }

  .day-total {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .day-header {
    height: 60px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-primary);
    position: sticky;
    top: 0;
    z-index: 5;
  }

  .day-header.today {
    background: var(--bg-secondary);
  }

  .day-name {
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    color: var(--text-secondary);
    letter-spacing: 0.5px;
  }

  .day-date {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    font-weight: 400;
    border-radius: 50%;
  }

  .day-date.today-date {
    background: #007aff;
    color: white;
    font-weight: 600;
  }

  .day-content {
    position: relative;
    height: calc(24 * 60px); /* 24 hours * 60px per hour */
    flex: 1;
  }

  /* Hour grid lines */
  .hour-lines {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 100%;
    pointer-events: none;
  }

  .hour-line {
    height: 60px;
    border-bottom: 1px solid var(--border-color);
  }

  /* Current time indicator */
  .current-time-line {
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: #ff3b30;
    z-index: 5;
    pointer-events: none;
  }

  .current-time-line::before {
    content: '';
    position: absolute;
    left: -4px;
    top: -3px;
    width: 8px;
    height: 8px;
    background: #ff3b30;
    border-radius: 50%;
  }

  /* Sessions */
  .sessions-container {
    position: absolute;
    top: 0;
    left: 4px;
    right: 4px;
    height: 100%;
  }

  .session-block {
    position: absolute;
    left: 0;
    right: 0;
    background: rgba(0, 122, 255, 0.15);
    border-left: 3px solid #007aff;
    border-radius: 4px;
    padding: 4px 8px;
    overflow: hidden;
    transition: background 0.15s;
  }

  .session-block.editing {
    background: rgba(0, 122, 255, 0.25);
    z-index: 10;
  }

  .session-block.in-progress {
    background: rgba(255, 149, 0, 0.15);
    border-left-color: #ff9500;
  }

  .session-content {
    font-size: 11px;
    line-height: 1.3;
  }

  .session-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 4px;
  }

  .session-tag {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 10px;
    font-weight: 500;
    border: none;
    max-width: 100px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.4;
  }

  .session-time {
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .session-time.editable,
  .session-comment.editable {
    cursor: pointer;
    border-radius: 3px;
    padding: 1px 2px;
    transition: background 0.1s;
  }

  .session-time.editable:hover,
  .session-comment.editable:hover {
    background: rgba(0, 122, 255, 0.1);
  }

  .session-time-edit {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-bottom: 2px;
  }

  .time-input-inline {
    width: 45px;
    padding: 2px 4px;
    border: none;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
    font-family: 'SF Mono', Monaco, 'Courier New', monospace;
    background: rgba(0, 122, 255, 0.15);
    color: var(--text-primary);
    text-align: center;
  }

  .time-input-inline:focus {
    outline: none;
    background: rgba(0, 122, 255, 0.25);
    box-shadow: inset 0 0 0 1px #007aff;
  }

  .comment-input-inline {
    width: 100%;
    padding: 2px 4px;
    border: none;
    border-radius: 3px;
    font-size: 10px;
    background: rgba(0, 122, 255, 0.15);
    color: var(--text-primary);
  }

  .comment-input-inline:focus {
    outline: none;
    background: rgba(0, 122, 255, 0.25);
    box-shadow: inset 0 0 0 1px #007aff;
  }

  .session-comment {
    color: var(--text-secondary);
    font-size: 10px;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .no-issue-tag {
    font-size: 9px;
    padding: 2px 6px;
    cursor: pointer;
    opacity: 0.7;
  }

  @media (prefers-color-scheme: light) {
    .no-issue-tag {
      background: rgba(0, 0, 0, 0.04);
      color: #666666;
      box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.12), inset 0 0 1px rgba(0, 0, 0, 0.08);
    }

    .no-issue-tag:hover {
      background: rgba(0, 0, 0, 0.08);
      opacity: 1;
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
      opacity: 1;
    }
  }

  /* Sync status badge */
  .sync-status-badge {
    position: absolute;
    bottom: 4px;
    right: 4px;
    font-size: 10px;
    opacity: 0.8;
  }

  .sync-status-badge .sync-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .sync-status-badge .sync-icon svg {
    display: block;
  }

  .sync-status-badge .sync-icon.synced {
    color: #666;
    opacity: 0.8;
  }

  .sync-status-badge .sync-icon.unsynced {
    color: #888;
    opacity: 0.7;
    animation: pulse 2s ease-in-out infinite;
  }

  .sync-status-badge .sync-icon.in-progress {
    color: #555;
    opacity: 0.7;
  }

  /* Conflict state - to be implemented when conflict detection is added */
  /* .sync-status-badge .sync-icon.conflict {
    color: #d32f2f;
    opacity: 1;
    cursor: pointer;
  }

  .sync-status-badge .sync-icon.conflict:hover {
    opacity: 0.8;
  } */

  @keyframes pulse {
    0%, 100% { opacity: 0.7; }
    50% { opacity: 0.4; }
  }

</style>
