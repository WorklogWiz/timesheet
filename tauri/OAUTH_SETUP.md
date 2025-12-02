# OAuth 2.0 Setup for Jira Integration

This app supports OAuth 2.0 (3-legged OAuth / 3LO) for secure Jira authentication without storing passwords.

## Prerequisites

You need to create an OAuth 2.0 app in the Atlassian Developer Console.

## Step 1: Create an Atlassian OAuth App

1. Go to [Atlassian Developer Console](https://developer.atlassian.com/console/myapps/)
2. Click **"Create"** → **"OAuth 2.0 integration"**
3. Fill in your app details:
   - **App name:** Timesheet (or your preferred name)
   - **App description:** Time tracking application

## Step 2: Configure OAuth Settings

1. In your app settings, click **"Permissions"**
2. Add the following **Jira API** scopes:
   - `read:jira-work` - Read work data
   - `read:jira-user` - Read user information
   - `write:jira-work` - Write work logs
   - `offline_access` - Get refresh tokens for long-term access

3. Click **"Authorization"** in the left sidebar
4. Add a callback URL:
   ```
   http://127.0.0.1:8080/callback
   ```
   ⚠️ **Important**: This exact URL must be configured in your OAuth app!

5. Save your settings

## Step 3: Get Your Credentials

1. In the app settings, find your:
   - **Client ID** (e.g., `abc123xyz...`)
   - **Client Secret** (click "Generate secret" if you haven't already)
2. **Copy both values** - you'll need them in the next step

## Step 4: Configure the Timesheet App

1. Open **Settings** in the Timesheet app
2. Scroll to **Jira Integration**
3. Click **"Configure"** or **"Edit Configuration"**
4. Select **"OAuth 2.0"** as the authentication type
5. Enter your OAuth credentials:
   - **Client ID:** Paste your Client ID
   - **Client Secret:** Paste your Client Secret
6. Click **"🔗 Connect with Atlassian OAuth"**

## Step 5: Authorize the App

When you click the connect button:

1. **Browser opens** → You'll be redirected to Atlassian's login page
2. **Log in** → Enter your Atlassian credentials
3. **Grant access** → Review and approve the requested permissions
4. **Redirect back** → Atlassian redirects you to `http://127.0.0.1:8080/callback`
5. **Success!** → You'll see a confirmation message, and the app will be configured

## What Happens Behind the Scenes

The OAuth flow works as follows:

1. App starts a **local HTTP server** on port 8080
2. App opens browser to **Atlassian authorization URL**
3. You authenticate and grant permissions
4. Atlassian redirects to `http://127.0.0.1:8080/callback?code=...`
5. App receives the **authorization code**
6. App exchanges code for **access token**
7. App fetches your **user info and Jira site**
8. Configuration is **automatically saved**

## Security

- **Access tokens** are stored in the macOS Keychain (service: `com.norn.timesheet.jira`)
- **Client secrets** are never exposed in the UI
- **CSRF protection** via random state parameter
- **Local server** only accepts localhost connections

## Troubleshooting

### "Failed to start callback server"
- Port 8080 is already in use
- Close any apps using port 8080 and try again

### "Authorization timeout"
- You didn't complete the authorization within 60 seconds
- Click the connect button again

### "Token exchange failed"
- Check your **Client ID** and **Client Secret** are correct
- Verify the **callback URL** in your OAuth app settings: `http://127.0.0.1:8080/callback`

### "No Jira sites found"
- Your Atlassian account doesn't have access to any Jira sites
- Make sure you're logged in to the correct account

### OAuth vs API Token

| Feature | OAuth 2.0 | API Token |
|---------|-----------|-----------|
| Setup complexity | More complex (requires OAuth app) | Simpler (just generate token) |
| Security | More secure (no password storage) | Secure (but requires token management) |
| Expiration | Automatic refresh (with refresh tokens) | Must manually regenerate |
| User experience | Browser-based login | Copy/paste token |
| Best for | Production, team apps | Personal use, quick setup |

## Recommended Approach

- **For personal use:** API Token is simpler and faster
- **For team/organization:** OAuth 2.0 is more secure and professional

## Additional Resources

- [Atlassian OAuth 2.0 Documentation](https://developer.atlassian.com/cloud/oauth/getting-started/implementing-oauth-3lo/)
- [OAuth 2.0 Security Best Practices](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-security-topics)
- [Jira REST API Documentation](https://developer.atlassian.com/cloud/jira/platform/rest/v3/)


