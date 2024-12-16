# Steinars notes


## Todo
- [ ] Derive keychain service name from the application name space name
- [ ] Define a separate Jira account for the integration tests `norn@balder.no` or something like that.
      I am not sure if we can use a fake name or need the name to be an actual email account somewhere.
## Setting the environment variables

```
export JIRA_HOST=https://norns.atlassian.com/jira
export JIRA_USER=norns@balder.no
export JIRA_TOKEN=$(security find-generic-password -s com.norns.timesheet.jira -a steinar.cook@gmail.com -w)
```

## How configuration parameters are loaded and merged with KeyChain

The configuration file is loaded from disk

```plantuml
control ApplicationRuntime
control "worklog::config" as wl
control "macos" as macos_sec

ApplicationRuntime -> wl : config::load()

activate wl
wl -> wl : app_config = read(&config_path)

alt macOS
      activate wl
      alt !secure_credentials::get_secure_token() && jira.has_valid_token()
        wl -> wl: create_configuration_file()
      end
      wl -> wl: merge_jira_token_from_keychain(app_config)  
      activate wl
        wl -> macos_sec : get_secure_token(service, user)
        activate macos_sec
        macos_sec --> wl : token
        deactivate macos_sec
      deactivate wl
      wl -> wl : app_config.jira.token = token
      deactivate wl
end
wl --> ApplicationRuntime : app_config: AppConfiguration 
deactivate wl
```

## Generic Data model for timesheet

```plantuml
@startuml
entity "User" {
    * id: UUID
    * name: String
    * email: String
    * role: String <<ENUM>>  // (e.g., "Admin", "Employee", "Manager")
    --
    + authenticate()
    + view_timesheets()
    + manage_timesheets()
}

entity "Project" {
    * id: UUID
    * name: String
    * description: String
    * start_date: Date
    * end_date: Date
    --
    + assign_users()
    + add_tasks()
    + view_timesheets()
}

entity "Task" {
    * id: UUID
    * project_id: UUID
    * name: String
    * description: String
    * status: String <<ENUM>>  // (e.g., "Open", "In Progress", "Closed")
    --
    + assign_user()
    + track_progress()
    + update_time_spent()
}

entity "Timesheet" {
    * id: UUID
    * user_id: UUID
    * task_id: UUID
    * date: Date
    * hours_worked: Float  // (e.g., 7.5 hours)
    * comments: String
    --
    + add_entry()
    + edit_entry()
    + submit_timesheet()
}

entity "Role" {
    * id: UUID
    * name: String // (e.g., "Admin", "Employee", "Manager")
    * permissions: String[]
    --
    + assign_to_user()
}

User "1" -- "0..*" Timesheet : "creates"
User "0..*" -- "0..*" Project : "assigned_to"
Project "1" -- "0..*" Task : "contains"
Task "1..*" -- "0..*" Timesheet : "logs"
Role "0..*" -- "1..1" User : "assigned"

@enduml
```