# Refactor sync command
```plantuml
component ApplicationRuntime {
    enum Operation {
        Add
        Del
        Codes
        Sync <<new>>
    }
    
    enum OperationResult {
        Added(Vec<LocalWorklog>)
        Deleted(String)
        IssueSummaries
        Synchronised()
    }
    class Impl {
        pub async fn execute(Operation)
    }
 }
 
 component "worklog/lib.rs" <<crate>> {
    enum Operation {
        Add(Add),
        Del(Del),
        Codes,
        Sync(Sync),
    }
    component operation <<module>> {
    component add.rs {
        struct Add {
            fields ...
            pub(crate) async fn execute() 
        }
    }
    component sync.rs {
        Struct Sync {
            fields ...
            pub(crate) async fn execute()
        }
    }

    
 }
 component cli <<crate>> {
    component cli <<module>> {
        enum Command {
            Add(Add),
            Del(Del),
            Status(Status),
            Config(Config),
            Codes,
            Sync(Synchronisation),
        }
        class Synchronisation
        class Add
        Command ..> Add
        Command ..> Synchronisation
    }
 }
        
 }

```

General execution flow:
```plantuml
participant test
participant "worklog::ApplicationRuntime" as Runtime
participant "worklog::operation::sync" as sync_cmd
participant Jira
participant WorklogStorage

test -> Runtime : execute(Operation::Sync(...))
Runtime -> sync_cmd : execute(runtime, cmd)
activate sync_cmd
sync_cmd -> sync_cmd : prepare_issue_keys()
activate sync_cmd
sync_cmd -> Jira: get_issue_summaries()
sync_cmd <-- Jira: Vec<IssueSummary>
deactivate
sync_cmd -> Jira : chuncked_worklogs()
sync_cmd <-- Jira : Vec<Worklog>
alt "All Users specified in CLI"
    sync_cmd -> sync_cmd : retain(current_user)
end 

sync_cmd -> Runtime : sync_jira_issue_information()
activate Runtime
    Runtime -> WorklogStorage : add_jira_issues()
    Runtime -> WorklogStorage : add_component()
    sync_cmd <-- Runtime
deactivate

Runtime <-- sync_cmd : Result(some stuff)
deactivate 
test <-- Runtime : Result(OperationResult)


```