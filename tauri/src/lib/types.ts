export interface TaskSession {
  id: string;
  issue_key: string | null; // Optional - can be assigned later
  start_date: string;
  end_date: string | null;
  comment: string | null;
  in_progress: boolean;
  synced_to_tracker: boolean; // Whether this has been synced to the issue tracker
  tracker_worklog_id: string | null; // Issue tracker's worklog ID if synced
  time_codes: TimeCode[];
}

export interface TimeCode {
  id: string;
  name: string;
  start_date: string;
  end_date: string;
}

export interface JiraConfig {
  url: string;
  user: string;
  auth_type: 'token' | 'oauth';
  token: string;
}

export interface OAuthConfig {
  client_id: string;
  client_secret: string;
}

export interface OAuthTokenResponse {
  access_token: string;
  refresh_token?: string;
  expires_in: number;
}

export interface JiraIssue {
  key: string;
  summary: string;
  from_cache: boolean;
}

export type Route = '/sessions' | '/week' | '/settings';

