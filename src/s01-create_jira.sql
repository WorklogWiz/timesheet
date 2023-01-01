-- set time zone 'CET';
-- show timezone;
create schema  if not exists jira;

create table jira.author
(
    id            serial primary key,
    account_id    varchar(64)
        CONSTRAINT unique_account_id UNIQUE,
    email_address varchar(255),
    display_name  varchar(255)
);

drop table if exists worklog;

create table jira.worklog
(
    id               varchar(32) primary key,
    account_id      varchar(64) references jira.author(account_id) on delete cascade ,
    created          timestamp with time zone,
    updated          timestamp with time zone,
    started          timestamp with time zone,
    timeSpent        varchar(16),
    timeSpentSeconds integer,
    issueId          varchar(32) constraint unique_issue_id UNIQUE
);
