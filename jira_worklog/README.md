# The Jira worklog utility

The `jira_worklog` utility allows you to enter your Jira worklog entries as easy and simple
as possible from the command line.

This utility will let you add your work log entries in less than 1s.

A quick status report for the last 30 days typically executes in less than 2-3 seconds, unless 
the Jira project contains thousands of entries. 

Disclaimer: Network latency and the response time of Jira is the main culprit of any delays 

<!-- TOC -->
* [The Jira worklog utility](#the-jira-worklog-utility)
  * [Installation](#installation)
    * [Using `curl` to verify your security token](#using-curl-to-verify-your-security-token)
    * [Notes on security](#notes-on-security)
  * [Examples](#examples-)
    * [Adding worklog entries](#adding-worklog-entries)
    * [Status of your worklog entries](#status-of-your-worklog-entries)
    * [Removing entries](#removing-entries)
    * [Debug](#debug)
<!-- TOC -->
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
### Using `curl` to verify your security token
Here is how you can retrieve data from Jira using the `curl` utility from the command line:
````shell
curl --request GET \
  --url 'https://autostore.atlassian.net/rest/api/2/myself' \
  --user '<your email here>@autostoresystem.com:<paste your security token here>' \
  --header 'Accept: application/json'
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
Version: 0.3.0
Issue    IssueId Id      Weekday Started                Time spent Comment
TIME-117 167111  304691  Mon     2024-08-19 14:42 +0200 00:30      
TIME-117 167111  305981  Wed     2024-08-28 08:36 +0200 01:00      
TIME-117 167111  306255  Wed     2024-08-28 11:22 +0200 03:00      
TIME-117 167111  309851  Tue     2024-09-03 08:00 +0200 07:30      Multi grid workshop in Haugesund
TIME-147 211874  309901  Mon     2024-09-02 08:00 +0200 07:30      
TIME-147 211874  309995  Wed     2024-09-04 08:00 +0200 07:30      PO role desc, DT architecture, time logging
TIME-147 211874  310089  Thu     2024-09-06 01:21 +0200 07:30      
TIME-147 211874  310499  Fri     2024-09-06 08:00 +0200 07:30      Admin work, DT migration planning
TIME-147 211874  310500  Fri     2024-09-06 08:00 +0200 03:00      jira_worklog
TIME-147 211874  310501  Sat     2024-09-07 08:00 +0200 03:00      jira_worklog
TIME-147 211874  310535  Sun     2024-09-08 07:43 +0200 01:00      Added monthly summary to worklog
TIME-40  85002   304588  Mon     2024-08-05 08:00 +0200 07:30      
TIME-40  85002   304589  Tue     2024-08-06 08:00 +0200 07:30      
TIME-40  85002   304590  Wed     2024-08-07 08:00 +0200 07:30      
TIME-40  85002   304591  Thu     2024-08-08 08:00 +0200 07:30      
TIME-40  85002   304592  Fri     2024-08-09 08:00 +0200 07:30      
TIME-40  85002   303933  Mon     2024-08-12 09:00 +0200 10:00      
TIME-40  85002   304329  Tue     2024-08-13 08:00 +0200 08:00      
TIME-40  85002   304330  Wed     2024-08-14 08:00 +0200 10:00      
TIME-40  85002   304331  Thu     2024-08-15 08:00 +0200 07:30      
TIME-40  85002   304431  Fri     2024-08-16 08:00 +0200 06:00      
TIME-40  85002   304593  Sun     2024-08-18 16:00 +0200 01:00      
TIME-40  85002   305844  Mon     2024-08-19 08:00 +0200 07:30      
TIME-40  85002   305845  Tue     2024-08-20 08:00 +0200 07:30      
TIME-40  85002   305846  Wed     2024-08-21 08:00 +0200 07:30      
TIME-40  85002   305847  Thu     2024-08-22 08:00 +0200 07:30      
TIME-40  85002   305848  Fri     2024-08-23 08:00 +0200 07:30      
TIME-40  85002   305849  Sun     2024-08-25 08:00 +0200 04:00      
TIME-40  85002   305850  Mon     2024-08-26 08:00 +0200 07:30      
TIME-40  85002   305852  Tue     2024-08-27 08:00 +0200 07:30      Strategy workshop
TIME-40  85002   305853  Tue     2024-08-27 08:00 +0200 03:00      Workshop part2 
TIME-40  85002   306261  Wed     2024-08-28 10:26 +0200 04:00      Meetings about admin
TIME-40  85002   309850  Thu     2024-08-29 08:00 +0200 07:30      
TIME-40  85002   307325  Fri     2024-08-30 08:20 +0200 07:30      Walk and talk and product ownership

Date       Day  time-40  time-147 time-117   Total
-------------- -------- -------- -------- --------
2024-08-05 Mon    07:30        -        -    07:30
2024-08-06 Tue    07:30        -        -    07:30
2024-08-07 Wed    07:30        -        -    07:30
2024-08-08 Thu    07:30        -        -    07:30
2024-08-09 Fri    07:30        -        -    07:30
-------------- -------- -------- -------- --------
ISO Week 32       37:30    00:00    00:00    37:30
============== ======== ======== ======== ========

Date       Day  time-40  time-147 time-117   Total
-------------- -------- -------- -------- --------
2024-08-12 Mon    10:00        -        -    10:00
2024-08-13 Tue    08:00        -        -    08:00
2024-08-14 Wed    10:00        -        -    10:00
2024-08-15 Thu    07:30        -        -    07:30
2024-08-16 Fri    06:00        -        -    06:00
2024-08-18 Sun    01:00        -        -    01:00
-------------- -------- -------- -------- --------
ISO Week 33       42:30    00:00    00:00    42:30
============== ======== ======== ======== ========

Date       Day  time-40  time-147 time-117   Total
-------------- -------- -------- -------- --------
2024-08-19 Mon    07:30        -    00:30    08:00
2024-08-20 Tue    07:30        -        -    07:30
2024-08-21 Wed    07:30        -        -    07:30
2024-08-22 Thu    07:30        -        -    07:30
2024-08-23 Fri    07:30        -        -    07:30
2024-08-25 Sun    04:00        -        -    04:00
-------------- -------- -------- -------- --------
ISO Week 34       41:30    00:00    00:30    42:00
============== ======== ======== ======== ========

Date       Day  time-40  time-147 time-117   Total
-------------- -------- -------- -------- --------
2024-08-26 Mon    07:30        -        -    07:30
2024-08-27 Tue    10:30        -        -    10:30
2024-08-28 Wed    04:00        -    04:00    08:00
2024-08-29 Thu    07:30        -        -    07:30
2024-08-30 Fri    07:30        -        -    07:30
-------------- -------- -------- -------- --------
ISO Week 35       37:00    00:00    04:00    41:00
============== ======== ======== ======== ========

Date       Day  time-40  time-147 time-117   Total
-------------- -------- -------- -------- --------
2024-09-02 Mon        -    07:30        -    07:30
2024-09-03 Tue        -        -    07:30    07:30
2024-09-04 Wed        -    07:30        -    07:30
2024-09-05 Thu        -    07:30        -    07:30
2024-09-06 Fri        -    10:30        -    10:30
2024-09-07 Sat        -    03:00        -    03:00
2024-09-08 Sun        -    01:00        -    01:00
-------------- -------- -------- -------- --------
ISO Week 36       00:00    37:00    07:30    44:30
============== ======== ======== ======== ========

CW 33    :    37:30
CW 34    :    42:30
CW 35    :    42:00
-------------------
August   :   122:00
===================

CW 36    :    41:00
-------------------
September:    41:00
===================
`````
### Removing entries
We all make mistakes every now then. To remove an entry you need to specify the 
`issueId or key` and the `worklog id`:
`````shell
# Rmoves a work log entry for issue TIME-94 with worklog id of 216626
jira_worklog del -i time-94 -w 216626
`````

### Debug
A log-file is created behind the scenes if you use the `--verbosity` option, which allows for debugging:
````shell
jira_worklog status -i time-40 -v debug
````

Output would look something like this:
````shell
jira_worklog status -i TIME-40 -v debug
Version: 0.2.7
Logging to /var/folders/ll/ywcp72091yv33vkts306qs0r0000gn/T/jira_worklog.log
Issue    IssueId      Id         Weekday Started                      Time spent
TIME-40  85002        304588     Mon     2024-08-05 08:00 +0200       07:30 
TIME-40  85002        304589     Tue     2024-08-06 08:00 +0200       07:30 
...... lots of data .....
````

You can specify one of `debug`, `info`, `warn` or `error`
