
# Jira Time Sheet

Rust learning project to extract and update hours logged in Jira.

The project consists of the following modules:

* `common` - common types shared between various 
* `jira_lib` - library with various functions to retrieve data from Jira
* `jira_worklog` - command line utility to register logged hours into Jira
* `jira_worklog_tui` - text user interface utility as an alternative to `jira_worklog`
* `local_repo` - Sqlite DBMS repository
* `rust-axum-backend` - attempt to create a backend for a web based interface
* `secure_credentials` - secure credentials for macOS clients
* `worklog_lib` - common functionality to be shared between the various clients

## How to build on MacOS

The script `build.sh` will compile all the binaries and upload them
to the OneDrive directory.

## Cross compiling to Windows on MacOS

How to cross compile from Mac to Windows:

```shell
brew install mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu
```

**Note** This was the first entry I found on Google in June 2023. The Windows executable
is rather large, so perhaps there is better way to do this.

Moved to autostore-tools repo on Sept 2, 2024

Here is an overview of the dependencies, extracted from `Cargo.toml`:

```plantuml
component common {
    component date
    component journal
    component config
}

component jira_lib
component jira_worklog
component jira_worklog_tui
component local_repo
component "rust-axum-backend" as Web
component secure_credentials
component worklog_lib

jira_worklog ..> jira_lib
jira_worklog ..> common
jira_worklog ..> worklog_lib
jira_worklog ..> local_repo

jira_worklog_tui ..> jira_lib
jira_worklog_tui ..> common

local_repo ..> common
local_repo ..> jira_lib

jira_lib ..> common

common ..> secure_credentials

worklog_lib ..> common
worklog_lib ..> jira_lib
worklog_lib ..> local_repo


```
