<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { api } from '../lib/api';
  import { preferences } from '../lib/stores';
  import type { JiraConfig, OAuthConfig } from '../lib/types';

  let syncResult = '';
  let testingNotification = false;

  // App preferences
  let use24HourFormat = false;
  preferences.subscribe(prefs => {
    use24HourFormat = prefs.use24HourFormat;
  });

  // Jira configuration
  let jiraConfigured = false;
  let showJiraForm = false;
  let jiraConfig: JiraConfig = {
    url: '',
    user: '',
    auth_type: 'token',
    token: '',
  };
  let originalToken = '';
  let testingConnection = false;
  let connectionResult = '';
  let savingConfig = false;

  // OAuth configuration
  let oauthConfig: OAuthConfig = {
    client_id: '',
    client_secret: '',
  };
  let oauthInProgress = false;
  let oauthResult = '';

  onMount(async () => {
    // Load Jira config
    await loadJiraConfig();
  });

  async function loadJiraConfig() {
    try {
      const config = await api.getJiraConfig();
      if (config) {
        jiraConfigured = true;
        jiraConfig = config;
        originalToken = config.token; // Store to detect changes
      }
    } catch (error) {
      console.error('Failed to load Jira config:', error);
    }
  }

  async function testConnection() {
    testingConnection = true;
    connectionResult = '';
    try {
      // Create a test config with the actual token
      const testConfig = {
        ...jiraConfig,
        token: jiraConfig.token === '***' ? '' : jiraConfig.token,
      };

      if (!testConfig.token) {
        connectionResult = 'Please enter a token or OAuth token first';
        return;
      }

      const result = await api.testJiraConnection(testConfig);
      connectionResult = result;
    } catch (error) {
      connectionResult = `Connection test failed: ${error}`;
    } finally {
      testingConnection = false;
    }
  }

  async function saveJiraConfig() {
    savingConfig = true;
    try {
      // Only save if token was actually changed (not the placeholder)
      const configToSave = {
        ...jiraConfig,
        token: jiraConfig.token === '***' ? originalToken : jiraConfig.token,
      };

      await api.saveJiraConfig(configToSave);
      jiraConfigured = true;
      showJiraForm = false;
      originalToken = jiraConfig.token;
      syncResult = 'Jira configuration saved successfully. Restart the app to use the new configuration.';
    } catch (error) {
      syncResult = `Failed to save configuration: ${error}`;
    } finally {
      savingConfig = false;
    }
  }

  async function removeJiraConfig() {
    if (!confirm('Are you sure you want to remove the Jira configuration?')) {
      return;
    }

    try {
      await api.removeJiraConfig();
      jiraConfigured = false;
      showJiraForm = false;
      jiraConfig = {
        url: '',
        user: '',
        auth_type: 'token',
        token: '',
      };
      originalToken = '';
      syncResult = 'Jira configuration removed. Restart the app to work in local-only mode.';
    } catch (error) {
      syncResult = `Failed to remove configuration: ${error}`;
    }
  }

  async function startOAuthFlow() {
    if (!oauthConfig.client_id || !oauthConfig.client_secret) {
      oauthResult = 'Please enter both Client ID and Client Secret';
      return;
    }

    oauthInProgress = true;
    oauthResult = 'Opening browser for authorization...';

    try {
      // Start OAuth flow - this will open the browser and wait for callback
      const authCode = await api.startOAuthFlow(oauthConfig);

      oauthResult = 'Exchanging authorization code for access token...';

      // Exchange auth code for access token
      const tokenResponse = await api.exchangeOAuthCode(authCode, oauthConfig);

      oauthResult = 'Getting user information...';

      // Get user info and Jira URL
      const userInfoJson = await api.getOAuthUserInfo(tokenResponse.access_token);
      const userInfo = JSON.parse(userInfoJson);

      // Fill in the form with OAuth info
      jiraConfig.url = userInfo.jiraUrl;
      jiraConfig.user = userInfo.email;
      jiraConfig.auth_type = 'oauth';
      jiraConfig.token = tokenResponse.access_token;

      oauthResult = `Successfully connected as ${userInfo.displayName}!`;
      connectionResult = `Connected to ${userInfo.jiraUrl}`;

      // Save the configuration
      await saveJiraConfig();
    } catch (error) {
      oauthResult = `OAuth failed: ${error}`;
    } finally {
      oauthInProgress = false;
    }
  }

  async function testNotification() {
    testingNotification = true;
    try {
      await invoke('test_sleep_notification');
    } catch (error) {
      console.error('Failed to test notification:', error);
    } finally {
      testingNotification = false;
    }
  }
</script>

<div class="settings-view">
  <h1>Settings</h1>

  <div class="settings-section">
    <h2>App Preferences</h2>

    <div class="form-group">
      <label class="checkbox-label">
        <input
          type="checkbox"
          bind:checked={use24HourFormat}
          on:change={() => preferences.setUse24Hour(use24HourFormat)}
        />
        Use 24-hour time format
      </label>
      <div class="help-text">
        When enabled, times will be displayed in 24-hour format (e.g., 13:00 instead of 1:00 PM)
      </div>
    </div>

    <div class="form-group">
      <h3>Notifications</h3>
      <button on:click={testNotification} disabled={testingNotification}>
        {testingNotification ? 'Sending...' : '🔔 Test Sleep Notification'}
      </button>
      <div class="help-text">
        Test how the notification will appear when your computer wakes from sleep
      </div>
    </div>
  </div>

  <div class="settings-section">
    <h2>Jira Integration</h2>

    {#if jiraConfigured && !showJiraForm}
      <div class="config-status">
        <div class="status-indicator connected"></div>
        <div>
          <div class="status-text">Connected to {jiraConfig.url}</div>
          <div class="status-subtext">User: {jiraConfig.user}</div>
        </div>
      </div>

      <div class="button-group">
        <button on:click={() => showJiraForm = true}>Edit Configuration</button>
        <button class="danger" on:click={removeJiraConfig}>Remove Configuration</button>
      </div>
    {:else if showJiraForm || !jiraConfigured}
      <div class="jira-form">
        <div class="form-group">
          <label for="jiraUrl">Jira URL:</label>
          <input
            id="jiraUrl"
            type="url"
            placeholder="https://your-domain.atlassian.net"
            bind:value={jiraConfig.url}
          />
        </div>

        <div class="form-group">
          <label for="jiraUser">Email / Username:</label>
          <input
            id="jiraUser"
            type="text"
            placeholder="your-email@example.com"
            bind:value={jiraConfig.user}
          />
        </div>

        <div class="form-group">
          <div class="group-label">Authentication Type:</div>
          <div class="radio-group">
            <label class="radio-label">
              <input
                type="radio"
                bind:group={jiraConfig.auth_type}
                value="token"
              />
              API Token (Recommended)
            </label>
            <label class="radio-label">
              <input
                type="radio"
                bind:group={jiraConfig.auth_type}
                value="oauth"
              />
              OAuth 2.0
            </label>
          </div>
        </div>

        {#if jiraConfig.auth_type === 'oauth'}
          <div class="oauth-section">
            <div class="help-text">
              You need to create an OAuth 2.0 (3LO) app in your Atlassian account first.
              <a href="https://developer.atlassian.com/console/myapps/" target="_blank">
                Create OAuth app
              </a>
            </div>

            <div class="form-group">
              <label for="oauthClientId">Client ID:</label>
              <input
                id="oauthClientId"
                type="text"
                placeholder="Enter your OAuth Client ID"
                bind:value={oauthConfig.client_id}
              />
            </div>

            <div class="form-group">
              <label for="oauthClientSecret">Client Secret:</label>
              <input
                id="oauthClientSecret"
                type="password"
                placeholder="Enter your OAuth Client Secret"
                bind:value={oauthConfig.client_secret}
              />
            </div>

            <div class="oauth-info">
              <strong>Important:</strong> Set your OAuth app's callback URL to:
              <code>http://127.0.0.1:8080/callback</code>
            </div>

            <button class="oauth-button" on:click={startOAuthFlow} disabled={oauthInProgress}>
              {oauthInProgress ? 'Connecting...' : '🔗 Connect with Atlassian OAuth'}
            </button>

            {#if oauthResult}
              <div class="result-message" class:error={oauthResult.includes('failed')}>
                {oauthResult}
              </div>
            {/if}
          </div>
        {:else}
          <div class="form-group">
            <label for="jiraToken">API Token:</label>
            <input
              id="jiraToken"
              type="password"
              placeholder="Enter your Jira API token"
              bind:value={jiraConfig.token}
            />
            <div class="help-text">
              <a href="https://id.atlassian.com/manage-profile/security/api-tokens" target="_blank">
                Generate API token
              </a>
            </div>
          </div>
        {/if}

        {#if jiraConfig.auth_type === 'token'}
          <div class="button-group">
            <button on:click={testConnection} disabled={testingConnection}>
              {testingConnection ? 'Testing...' : 'Test Connection'}
            </button>
            <button class="primary" on:click={saveJiraConfig} disabled={savingConfig}>
              {savingConfig ? 'Saving...' : 'Save Configuration'}
            </button>
            {#if jiraConfigured}
              <button on:click={() => showJiraForm = false}>Cancel</button>
            {/if}
          </div>
        {/if}

        {#if connectionResult}
          <div class="result-message" class:error={connectionResult.includes('failed')}>
            {connectionResult}
          </div>
        {/if}
      </div>
    {/if}

    {#if syncResult}
      <div class="result-message" class:error={syncResult.includes('failed')}>
        {syncResult}
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-view {
    padding: 20px;
    max-width: 800px;
    margin: 0 auto;
    height: 100%;
    overflow-y: auto;
  }

  h1 {
    margin: 0 0 24px 0;
    font-size: 24px;
    font-weight: 600;
  }

  .settings-section {
    margin-bottom: 32px;
    padding-bottom: 24px;
    border-bottom: 1px solid var(--border-color);
  }

  .settings-section:last-child {
    border-bottom: none;
  }

  h2 {
    margin: 0 0 16px 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  label,
  .group-label {
    font-weight: 500;
    min-width: 100px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-weight: 500;
    min-width: unset;
  }

  .checkbox-label input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }

  .help-text {
    margin-top: 6px;
    margin-left: 26px;
    font-size: 13px;
    color: var(--text-secondary);
  }

  input[type="text"],
  input[type="url"],
  input[type="password"] {
    padding: 6px 12px;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    background: var(--bg-primary);
    font-size: 14px;
    flex: 1;
  }

  .config-status {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    background: var(--bg-secondary);
    border-radius: 8px;
    margin-bottom: 16px;
  }

  .status-indicator {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #30d158;
  }

  .status-text {
    font-weight: 500;
    color: var(--text-primary);
  }

  .status-subtext {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  .jira-form {
    padding: 16px;
    background: var(--bg-secondary);
    border-radius: 8px;
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    margin-bottom: 6px;
    font-weight: 500;
    font-size: 13px;
    color: var(--text-primary);
  }

  .radio-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: auto;
    font-weight: normal;
    cursor: pointer;
  }

  .radio-label input[type="radio"] {
    cursor: pointer;
  }

  .help-text {
    margin-top: 4px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .help-text a {
    color: #007aff;
    text-decoration: none;
  }

  .help-text a:hover {
    text-decoration: underline;
  }

  .button-group {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  button {
    padding: 8px 16px;
    background: var(--button-bg);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    transition: background 0.15s;
  }

  button:hover:not(:disabled) {
    background: var(--button-hover-bg);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  button.primary {
    background: #007aff;
    color: white;
    border-color: #007aff;
  }

  button.primary:hover:not(:disabled) {
    background: #0051d5;
  }

  button.danger {
    color: #ff3b30;
  }

  .result-message {
    margin-top: 16px;
    padding: 12px;
    background: var(--success-bg);
    border: 1px solid var(--success-border);
    border-radius: 6px;
    font-size: 13px;
  }

  .result-message.error {
    background: var(--error-bg);
    border-color: var(--error-border);
    color: var(--error-text);
  }

  .oauth-section {
    background: rgba(0, 122, 255, 0.05);
    padding: 16px;
    border-radius: 8px;
    margin-top: 12px;
  }

  .oauth-info {
    padding: 12px;
    background: var(--bg-primary);
    border-radius: 6px;
    margin: 12px 0;
    font-size: 13px;
  }

  .oauth-info code {
    display: block;
    padding: 6px 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    font-family: 'SF Mono', Monaco, monospace;
    font-size: 12px;
    margin-top: 6px;
  }

  .oauth-button {
    width: 100%;
    padding: 12px;
    background: #0052cc;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
  }

  .oauth-button:hover:not(:disabled) {
    background: #0747a6;
  }

  .oauth-button:disabled {
    opacity: 0.6;
  }
</style>
