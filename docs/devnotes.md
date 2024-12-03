# Developer notes

## Migration

- How long do we support migration to the dbms journal from the csv journal?
- When detecting whether migration is needed, we should only tell the user when that is needed.
  - We detect an old journal file (Migration needed)
  - If we detect an old journal and the new at the same time, then something wrong has happened. Let the use decide what to do?
  - If neither of these conditions hold, we skip the migration silently. This should be done in the `ApplicationRuntime`?

## Configuration management

- Today there are a lot of exposed methods for configuration handling. Should this all be moved to the `ApplicationRuntime` inside the `worklog` library, so all executables can reuse the same handling?

## Error handling

- There are `panics!` and `process.exit(code)` inside of the libraries. This should all be ported to `Errors` so that this can be handled accordingly in the different apps. `cli` wants to give exit codes, `tui` wants to display popups? and the `server` will want to translate to http errors.

## Testing

- The coverage is rather low and it is hard to automate integration tests as they need to run towards a real Jira instance. Try to mock the `Jira` client so that we can test all kinds of cases. Also, use in-memory databases to quickly test the `LocalWorklog` interface.
