# The Jira worklog utility

The `jira_worklog` utility allows you to enter your Jira worklog entries as easy and simple
as possible from the command line.

This utility will let you add your work log entries in less than 1s.

A quick status report for the last 30 days typically executes in less than 2-3 seconds.


````shell
# Add one day of work to Jira issue TIME-94
jira_worklog add -i time-94 -d 1d

# Give me status for the last 30 days for time-94 and time-40
jira_worklog status - time-94 time-40
````

See the detailed examples below for more details.

## Installation
Once you have downloaded and installed `jira_worklog` in your path:
 1. Obtain a Jira API security token from:
    1. Log in to Jira
    2. Click on the picture of yourself in the upper right corner ("Your profile and settings")
    3. Click "Manage account"
    4. Click "Security" ![](images/jira_security.png)
    5. Choose "Create and manage API tokens" allmost at the bottom of the page
    6. Click "Create your API token" and copy the token to your clip board
2. Execute this command to create the configuration file:
    ````shell
   # Creates the configuration file and stores your credentials in it
    jira_worklog config --user steinar.cook@autostoresystem.com --token vbF**************E3
    ````

### Notes on security
The configuration file is stored without encryption in a location, which depends on the operating system you are using.
See the table below for details.

If you think your machine has been compromised, go to Jira account ira and "Revoke" the API key.

You can remove your local configuration file using the command: `jira_worklog config --remove`

|Operating system | Config file location                                               |
|-------|--------------------------------------------------------------------|
|MacOs: | `/Users/steinar/Library/Preferences/com.autostore.jira_worklog`    |
|Windows: | `C:\Users\Alice\AppData\Roaming\com.autostore\jira_worklog\config` |
|Linux: | `/home/steinar/.config/jira_worklog`                               |

## Examples 

Here are some examples on how to use the utility.

### Adding worklog entries

````shell
# Registers 1 hour of work on TIME-94 with a comment
# The starting point will be current time less 1 hour
jira_worklog add -i time-94 -d 1h -c "I did some great work for AutoStore"


# Registers 1 hour of work on TIME-94 at 11:00 today without a comment
jira_worklog add -i time-94 -d 1h -s 11:00 

# Registers 1 day of work (7.5h) on TIME-94, starting at 08:00 today, no comments
jira_worklog add -i time-94 -d 1d

# Registers 1 day (7.5 hours) of work starting at 08:00 today with no comment
jira_worklog add -i time-94 -d 1d 

#
# Add 1d of work last friday, 1d of work on last thursday, 4h of work 
# last Wednesday and 1,5h on last Tuesday
jira_worklog add -i time-94 -d Fri:1d Thu:1d Wed:4h Tue:1,5h
````

Given this command:
`````shell
jira_worklog add -i time-94 -d 13h -c "Meetings and managerial work"
`````
You will get a receipt looking something like this:
`````shell
Adding single entry
Using these parameters as input:
        Issue: TIME-94
        Started: 2023-06-05T08:12:31.467244-06:00  (computed)
        Duration: 46800s
        Comment: Meetings and managerial work
Added work log entry Id: 217258 Time spent: 1d 5h 30m Time spent in seconds: 46800 Comment: Meetings and managerial work
`````

### Status of your worklog entries

````shell
#
# Shows the status from a given date
jira_worklog status -i time-94 -a 2023-05-01
````

This would give you something like this:
`````shell
Issue    IssueId      Id         Weekday Started                      Time spent
TIME-94  125425       215834     Tue     2023-05-02 11:04 -0600       08:30 
TIME-94  125425       215835     Wed     2023-05-03 09:40 -0600       09:54 
TIME-94  125425       215836     Thu     2023-05-04 14:04 -0600       05:30 
TIME-94  125425       215837     Fri     2023-05-05 11:49 -0600       07:45 
TIME-94  125425       215830     Mon     2023-05-08 12:03 -0600       07:30 
TIME-94  125425       215831     Tue     2023-05-09 09:54 -0600       09:39 
TIME-94  125425       215832     Wed     2023-05-10 12:03 -0600       07:30 
TIME-94  125425       215833     Thu     2023-05-11 13:03 -0600       06:30 
TIME-94  125425       215825     Tue     2023-05-23 09:03 -0600       10:00 
TIME-94  125425       215826     Wed     2023-05-24 09:00 -0600       10:30 
TIME-94  125425       214472     Thu     2023-05-25 08:00 -0600       09:30 
TIME-94  125425       214471     Fri     2023-05-26 08:00 -0600       12:30 
TIME-94  125425       214470     Sat     2023-05-27 08:00 -0600       03:30 
TIME-94  125425       215824     Sun     2023-05-28 15:02 -0600       04:00 
TIME-94  125425       216572     Mon     2023-05-29 08:00 -0600       01:00 
TIME-94  125425       215827     Tue     2023-05-30 08:00 -0600       14:21 
TIME-94  125425       215828     Wed     2023-05-31 08:00 -0600       14:15 
TIME-94  125425       215829     Thu     2023-06-01 06:47 -0600       12:45 
TIME-94  125425       216585     Fri     2023-06-02 13:08 -0600       07:00 

CW Date       Day Duration 
18 2023-05-02 Tue 08:30   
18 2023-05-03 Wed 09:54   
18 2023-05-04 Thu 05:30   
18 2023-05-05 Fri 07:45   
-----------------------
ISO week 18, sum: 31:39 
=======================

19 2023-05-08 Mon 07:30   
19 2023-05-09 Tue 09:39   
19 2023-05-10 Wed 07:30   
19 2023-05-11 Thu 06:30   
-----------------------
ISO week 20, sum: 31:09 
=======================

21 2023-05-23 Tue 10:00   
21 2023-05-24 Wed 10:30   
21 2023-05-25 Thu 09:30   
21 2023-05-26 Fri 12:30   
21 2023-05-27 Sat 03:30   
21 2023-05-28 Sun 04:00   
-----------------------
ISO week 21, sum: 50:00 
=======================

22 2023-05-29 Mon 01:00   
22 2023-05-30 Tue 14:21   
22 2023-05-31 Wed 14:15   
22 2023-06-01 Thu 12:45   
22 2023-06-02 Fri 07:00   
-----------------------
ISO week 22, sum: 49:21 
=======================


May       142:24
`````
### Removing entries
We all make mistakes every now then. To remove an entry you need to specify the 
`issueId or key` and the `worklog id`:
`````shell
# Rmoves a work log entry for issue TIME-94 with worklog id of 216626
jira_worklog del -i time-94 -w 216626
`````

