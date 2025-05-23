# The Time Sheet utility

The `timesheet` utility allows you to enter your Jira worklog entries as easy and simple
as possible from the command line. The main objective for this utility is speed. If you don't like command line applications, don't use this tool :-)

This utility will let you add your work log entries in less than 1 second.

A quick status report for the last 30 days typically executes in less than 1 second.

All entries added to Jira will also be written to a local Sqlite database, which allows reports to be generated very fast.

This database is also used for status reports.

The local database may be synchronised with Jira using the `sync` sub command.

Disclaimer: Network latency and the response time of Jira is the main culprit of any delays

<!-- TOC -->
* [The Time Sheet utility](#the-time-sheet-utility)
  * [Installation](#installation)
    * [Using `curl` to verify your security token](#using-curl-to-verify-your-security-token)
    * [Installing on macOS](#installing-on-macos)
    * [Notes on security](#notes-on-security)
      * [macOS specifics for the Jira Security token](#macos-specifics-for-the-jira-security-token)
      * [Removing local configuration files](#removing-local-configuration-files)
  * [How to specify the duration](#how-to-specify-the-duration)
    * [Local database file](#local-database-file)
  * [Re-installing a new version](#re-installing-a-new-version)
  * [Examples](#examples)
    * [Adding worklog entries](#adding-worklog-entries)
    * [Status of your worklog entries](#status-of-your-worklog-entries)
    * [Create a status report from most used time codes](#create-a-status-report-from-most-used-time-codes)
    * [Removing entries](#removing-entries)
    * [Synchronising the local database with Jira](#synchronising-the-local-database-with-jira)
    * [Listing all available time codes](#listing-all-available-time-codes)
    * [Debug](#debug)
  * [Creating reports with SQL](#creating-reports-with-sql)
<!-- TOC -->

````shell
# Add one day of work to Jira issue TIME-94
timesheet add -i time-94 -d 1d

# Give me a status of all work log entries for the last 30 days
timesheet status

# Give me status for the last 30 days for time-94 and time-40
timesheet status -i time-94 time-40
````

See the detailed examples below for more details.

## Installation

Once you have downloaded and installed `timesheet` in your **path**.
Like, for instance, `$HOME/.local/bin`:

1. Obtain a Jira API security token from:

    1. Log in to Jira
    1. Click on the picture of yourself in the upper right corner ("Your profile and settings")
    1. Click "Manage account"
    1. Click "Security" ![Jira Security Screenshot](images/jira_security.png)
    1. Choose "Create and manage API tokens" almost at the bottom of the page
    1. Click "Create your API token" and copy the token to your clip board
2. Execute this command to create the configuration file:

    ````shell
   # Creates the configuration file and stores your credentials in it
    timesheet config --user me@whereever.com --token vbF**************E3
    ````

### Using `curl` to verify your security token

Here is how you can retrieve data from Jira using the `curl` utility from the command line:

````shell
curl --request GET \
  --url 'https://myinstance.atlassian.net/rest/api/2/myself' \
  --user 'me@whereever.com:<paste your security token here>' \
  --header 'Accept: application/json'
````

### Installing on macOS

There are some extra security built into the macOS which prevents you from running potential malware.
Consequently, you will see this error message if you attempt to run `timesheet`:

![macOS Unidentified Developer Screenshot](images/macOS_error_unidentified_dev.png)

To fix this:

```shell
# Move to the directory where you installed
cd [to_the_directory_where_you_have_installed_timesheet]

chmod a+rx ./timesheet && xattr -d com.apple.quarantine ./timesheet
```

This should solve the problem.

### Notes on security

The configuration file is stored without encryption in a location,
which depends on the operating system you are using.
See the table below for details.

If you think your machine has been compromised, go to Jira account ira and "Revoke" the API key.

#### macOS specifics for the Jira Security token

On macOS, the Jira Security Access Token is stored in the built-in KeyChain.

When `timesheet` attempts to access your macOS Keychain, this window will pop up.
It is a good idea to press `Always Allow` to save you some time :-)

![KeyChain Prompt](images/keychain_prompt.png)

Here is a neat command to work with security tokens and passwords
on macOS:

````shell
# This will list the entire contents of the `timesheet` entry from the keychain
security find-generic-password -s com.norns.timesheet -a your.name@company.com -g
````

#### Removing local configuration files

You can remove your local configuration file using the command: `timesheet config --remove`

| Operating system     | Config file location                                                    |
|----------------------|-------------------------------------------------------------------------|
| macOS:               | `/Users/${USER}/Library/Preferences/com.norns.timesheet`         |
| Windows:             | `C:\Users\%USERNAME%\AppData\Roaming\com.norns\timesheet\config` |
| Linux:               | `/home/${USER}/.config/timesheet`                                    |

Note! For macOS: The Jira Security Access token stored in the Keychain, will not be deleted

## How to specify the duration

You can specify the duration of your work using weeks, days, hours and minutes.

The syntax is pretty straight forward, you simply specify a number followed by the unit.
To combine units, simply concatenate them. The formal syntax is described below.
Note that `<number>` represents any positive number using either `,` or `.` as the decimal separator.
However, you may not specify fractions of minutes (for obvious reasons) :

````shell
<number>w<number>d<number>h<integer number>m
````

Here is an example using all the possible options:

````shell
# Specify a duration of 1,5 week, 2,5 days, 5,25 hours and 30min like this
timesheet add -i time-158 -d 1,5w2,5d5,25h30min
````

### Local database file

The local database file can be found here:

| Operating System | Local Sqlite database file                                                       |
|------------------|----------------------------------------------------------------------------------|
| macOS            | /Users/${USER}/Library/Application Support/com.norns.timesheet/worklog.db |
| Windows          | C:\Users\%USERNAME%\AppData\Roaming\timesheet\worklog.db                      |
| Linux            | /home/${LOGNAME}/.local/share/timesheet/worklog.db                            |

NOTE! I have neither access to a Windows nor a Linux system, so the specified paths might not be correct.

## Re-installing a new version

The database (*Sqlite*) file, can be removed and re-created at any time.

Simply remove the local database file and run the `sync` command again, see
[Synchronising the local database with Jira](#synchronising-the-local-database-with-jira)

## Examples

Here are some examples on how to use the utility.

### Adding worklog entries

````shell
# Registers 1 hour of work on TIME-94 with a comment
# The starting point will be current time less 1 hour
timesheet add -i time-94 -d 1h -c "I did some great work"


# Registers 1 hour of work on TIME-94 at 11:00 today without a comment
timesheet add -i time-94 -d 1h -s 11:00

# Registers 1 day of work (7.5h) on TIME-94, starting at 08:00 today, no comments
timesheet add -i time-94 -d 1d

# Registers 1 day (7.5 hours) of work starting at 08:00 today with no comment
timesheet add -i time-94 -d 1d

#
# Add 1d of work last friday, 1d of work on last thursday, 4h of work
# last Wednesday and 1,5h on last Tuesday
timesheet add -i time-94 -d Fri:1d Thu:1d Wed:4h Tue:1,5h
````

Given this command:

`````shell
timesheet add -i time-94 -d 13h -c "Meetings and managerial work"
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
timesheet status -i time-40 time-147 time-117 -a 2023-05-01
````

### Create a status report from most used time codes

If you omit the `--issue` option, a list of unique time codes will
be obtained from the local journal on your machine.

```shell
timesheet status
```

### Removing entries

We all make mistakes every now then. To remove an entry you need to specify the
`issueId or key` and the `worklog id`:

````shell
# Removes a work log entry for issue TIME-94 with worklog id of 216626
timesheet del -i time-94 -w 216626
````

The entry will also be removed from the local journal.

### Synchronising the local database with Jira

To ensure that your local database reflects the current content in Jira, you may use the sub-command `sync`.
All unique time codes found in the local database, will be synchronised with Jira by downloading the data from
Jira and replacing the data in the local database.

````shell
# Synchronise all time codes found in the local database going back 30 days
timesheet sync

# Synchronise the specified time codes going after the supplied start date
timesheet sync -i time-155 -s 2024-10-01

# Synchronise multiple time codes
timesheet sync -i time-155 -i time-166
````

The output looks something like this:

````shell
Old journal not found so return
Synchronising work logs for these issues:
        time-155
        time-166
Synchronising 1 entries for time code time-155
Synchronising 3 entries for time code time-166
````

### Using timers to log work

As of version 0.12 you can start a timer, which will log elapsed time until you stop it.
When you stop the timer, the elapsed time will be logged and uploaded to Jira automatically.

#### Start a timer
The following will start a timer for the issue EMG-2 with the comment "I am going to refactor some code"

```shell
timesheet start -i EMG-2 -c "I am going to refactor some code"
```

If you forgot to start the timer when you started working on a new issue, you can specify the
time explicitly using the `-s` option. 

I started working at 08:00 this morning, and sometime before lunch I realised I forgot to 
start the timer. Here is how to start a timer going back in time: 
```shell
timesheet start -i EMG-2 -c "Some optional comment" -s 08:00
```

#### Stopping a timer

To stop the current active timer, use the `stop` sub-command:
```shell
timesheet stop
```

To replace the comment before uploading to Jira:

```shell
timesheet stop -c "Reworked the module Foo Bar"
```

So you forgot to stop the timer and it is still running? 
Here is how you can override the stop time:

```shell
timesheet stop -s 14:00
```

Feel free to combine the `-c` and `-s` options:
```shell
timesheet stop -s 14:00 -c "Fixed all the bugs"
```

### Listing all available time codes

If you want a complete list of all the available time codes:

```shell
timesheet codes
```

### Debug

A log-file is created behind the scenes if you use the `--verbosity` option, which allows for debugging:

````shell
timesheet status -i time-40 -v debug
````

Output would look something like this:

````shell
timesheet status -i TIME-40 -v debug
Version: 0.2.7
Logging to /var/folders/ll/ywcp72091yv33vkts306qs0r0000gn/T/timesheet.log
Issue    IssueId      Id         Weekday Started                      Time spent
TIME-40  85002        304588     Mon     2024-08-05 08:00 +0200       07:30
TIME-40  85002        304589     Tue     2024-08-06 08:00 +0200       07:30
...... lots of data .....
````

You can specify one of `debug`, `info`, `warn` or `error`

## Creating reports with SQL
