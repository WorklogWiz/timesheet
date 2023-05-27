# The Jira worklog utility

The `jira_worklog` utility allows you to enter your worklog as easy and simple
as possible.

Here are some examples:

### Register 1 hour of work 
````shell
# Registers 1 hour of work on TIME-94 with a comment
# The starting point will be current time less 1 hour
jira_worklog add -i time-94 --duration 1h -c "I did some great work for AutoStore"


# Registers 1 hour of work on TIME-94 at 11:00 today without a comment
jira_worklog add -i time-94 -d 1h -s 11:00 

# Registers 1 day of work (7.5h) on TIME-94, starting at 08:00 today, no comments
jira_worklog add -i time-94 -d 1d

# Registers 1 day (7.5 hours) of work starting at 08:00 today with no comment
jira_worklog add -i time-94 -d 1d 

# Registers 3 days of work distributed over 3 days starting at now minus 3 days, 
# i.e. we go 3 days back and register 7.5 hours of work each day
jira_worklog add -i time-94 -d 3d -c "Job took a long time" 

````

