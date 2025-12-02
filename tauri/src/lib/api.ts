import { invoke } from '@tauri-apps/api/core';
import type { TaskSession, TimeCode, JiraConfig, OAuthConfig, OAuthTokenResponse, JiraIssue } from './types';

export const api = {
  // Timer commands
  startTimer: (issueKey?: string) => invoke<string>('start_timer', { issueKey: issueKey || null }),
  stopTimer: () => invoke<TaskSession>('stop_timer'),
  getActiveTimer: () => invoke<TaskSession | null>('get_active_timer'),
  getCurrentTime: () => invoke<string>('get_current_time'),
  isInProgress: () => invoke<boolean>('is_in_progress'),

  // Session commands
  getAllSessions: () => invoke<TaskSession[]>('get_all_sessions'),
  getSession: (id: string) => invoke<TaskSession | null>('get_session', { id }),
  addSession: (startDate: string, endDate: string | null, issueKey: string | null, comment: string | null) =>
    invoke<string>('add_session', { startDate, endDate, issueKey, comment }),
  updateSession: (id: string, startDate: string, endDate: string | null, issueKey: string | null, comment: string | null) =>
    invoke<void>('update_session', { id, startDate, endDate, issueKey, comment }),
  deleteSession: (id: string) => invoke<void>('delete_session', { id }),
  assignIssueToSession: (sessionId: string, issueKey: string) =>
    invoke<void>('assign_issue_to_session', { sessionId, issueKey }),

  // TimeCode commands
  addTimeCode: (sessionId: string, timeCode: TimeCode) =>
    invoke<void>('add_time_code', { sessionId, timeCode }),
  updateTimeCode: (timeCode: TimeCode) => invoke<void>('update_time_code', { timeCode }),
  deleteTimeCode: (id: string) => invoke<void>('delete_time_code', { id }),

  // Jira sync
  syncToJira: () => invoke<number>('sync_to_jira'),

  // Jira configuration
  getJiraConfig: () => invoke<JiraConfig | null>('get_jira_config'),
  saveJiraConfig: (config: JiraConfig) => invoke<void>('save_jira_config', { config }),
  testJiraConnection: (config: JiraConfig) => invoke<string>('test_jira_connection', { config }),
  removeJiraConfig: () => invoke<void>('remove_jira_config'),

  // OAuth flow
  startOAuthFlow: (oauthConfig: OAuthConfig) => invoke<string>('start_oauth_flow', { oauthConfig }),
  exchangeOAuthCode: (code: string, oauthConfig: OAuthConfig) =>
    invoke<OAuthTokenResponse>('exchange_oauth_code', { code, oauthConfig }),
  getOAuthUserInfo: (accessToken: string) => invoke<string>('get_oauth_user_info', { accessToken }),

  // Jira issues
  getAllJiraIssues: () => invoke<JiraIssue[]>('get_all_jira_issues'),
};

// Standalone API functions for convenience
export async function getActiveTimer(): Promise<TaskSession | null> {
  return api.getActiveTimer();
}
