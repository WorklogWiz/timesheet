# Steinars notes

<!-- TOC -->
* [Steinars notes](#steinars-notes)
  * [Todo](#todo)
  * [Repostiory pattern](#repostiory-pattern)
  * [AI Assistant's proposal](#ai-assistants-proposal)
  * [Setting the environment variables](#setting-the-environment-variables)
  * [How configuration parameters are loaded and merged with KeyChain](#how-configuration-parameters-are-loaded-and-merged-with-keychain)
  * [Generic Data model for timesheet](#generic-data-model-for-timesheet)
    * [Mapping Rust concepts to UML](#mapping-rust-concepts-to-uml)
  * [Organising code to provide maximum encapsulation](#organising-code-to-provide-maximum-encapsulation)
  * [DDD architecture](#ddd-architecture)
<!-- TOC -->
## Todo
- [ ] Derive keychain service name from the application name space name
- [ ] Define a separate Jira account for the integration tests `norn@balder.no` or something like that.
      I am not sure if we can use a fake name or need the name to be an actual email account somewhere.

## Start and stop timers
A timer can be started and stopped using the `timesheet` command.
Only a single timer can be active at a time.
```shell
# Starts a new timer assuming that <issue-key> exists in the Jira instance 
timesheet start -i <issue-key>  [-s start time]
# Stops the current timer, raises a warning if no timer is active
timesheet stop
```
## Repostiory pattern

```plantuml
frame main {
}
package dbms <<module>> {
    struct "sqlite" <<file>> {
        fn create_connection() -> Result<Connection>
    }
}

package repository <<module>> {
    frame "mod rs" {
        interface UserRepository <<trait>> {
          create_user(user: &User)
          find_user(id: i32)
          find_all_users()
          update_user(user: &User)
          delete_user(id: i32)
        }
    }
    frame "user_repo" {
        entity SqliteUserRepository {
            impl UserRepository for SqliteUserRepository {}
        }
    }
    SqliteUserRepository -up-|> UserRepository
}

package domain <<module>> {
    entity Issue <<Entity>> {
        id: String,
        name: String,
        email: String
        --
    }
    entity User {}
    entity WorkLog {}
    entity Component {}
    entity IssueComponent {}
    
}

package service <<module>> {
    frame "user_service" {
        entity UserService<R: UserRepository> {
            impl<R: UserRepository> UserService<R> {
                create_new_user(name, email, ..)
            }
        }
        UserService -> UserRepository
    }
}

```
## AI Assistant's proposal

```plantuml
participant main
participant "dbms::sqlite" as sqlite
participant "repository::user_repo::\nSqliteUserRepository" as user_repo
participant "service::user_service::\nUserService" as user_service

main -> sqlite : create_connection()
main <-- sqlite : connection
|||
main -> user_repo: new(connection)
main <-- user_repo: user_repo
|||
main -> user_service : new(user_repo)
main <-- user_service : user_service
|||
main -> user_service : create_new_user(user)
main <-- user_service : Result<User>

```
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

### Mapping Rust concepts to UML
* crate => component
* struct => struct
* trait => interface
* impl => methods in struct or class
* module => *package* or subcomponent
* sub module => nested package or *subcomponent*

- frame
- package = crate
- folder
- component 

## Organising code to provide maximum encapsulation
 - Each module must only expose their own domain structures. I.e.
   - `worklog` layer must not expose any structures related to `Jira` or the underlying `repo` module 
```plantuml
component cli <<crate>> {
    frame main {
    }
    component cli {
        enum Command {
            Add(Add),
            Del(Del),
            Status(Status),
            Config(Config),
            Codes,
            Sync(Synchronisation),
        }
    }
}

component jira {
    struct Jira {
        fn get()
    }
    component "models" {
        package core {
        }
        package issue {
        }
        package project {
        }
        package user {
        }
        package settings {
        }
        package worklog <<Jira>>{
        }
    }
}
component "worklog" <<crate>> {
    struct ApplicationRuntime {
    }
    package dbms <<module>> {
          struct "sqlite" <<file>> {
              fn create_connection() -> Result<Connection>
          }
    }
    package repository {
    }
    package service {
    }
    package types {
    
    }
    package operation {
    }
    package config {
    }
    package error {
        enum WorklogError {
        }
    }
    package types {
    }
}

cli -down-> worklog
worklog -down-> jira

```

## DDD architecture

```plantuml

component jira <<crate>> {
    component client {
    }
    component error {
    }
    component types {
        component user_dto.rs {
        }
        component issue_dto.rs {
        }
    }
}

component repository <<module>> {
    component user_repo.rs {
    }
    component entities {
        component user_entity.rs {
        }
    }
    component error {
    }
}

component worklog <<crate>> {
    component user_service.rs {
    }
    component conversion.rs {
    }
    component error.rs {
    }
}

```