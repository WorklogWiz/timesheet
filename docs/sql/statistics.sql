SELECT component.name                              AS component_name,
       sum(worklog.time_spent_seconds) / 3600      as hours,
       SUM(worklog.time_spent_seconds) % 3600 / 60 AS minutes
FROM issue
    JOIN
    worklog ON issue.issue_key = worklog.issue_key
    left outer join
    issue_component ON issue.issue_key = issue_component.issue_key
    left outer JOIN
    component ON issue_component.component_id = component.id
group by component.name
order by hours desc, minutes desc;

/* Total number of hours spent */
select sum(worklog.time_spent_seconds) as seconds,
       sum(worklog.time_spent_seconds) / 3600 as hours,
       sum(worklog.time_spent_seconds) % 3600 / 60 as minutes
    from worklog;

select *
from worklog
where issue_key in ('KT-1892', 'KT-2759')
  and date(worklog.started) = date('now');

SELECT  c.name,
        sum(time_spent_seconds),
        sum(time_spent_seconds) / 3600 as hours,
        sum(time_spent_seconds / 60) % 60 as minutes

FROM main.issue
    join worklog ON issue.issue_key = worklog.issue_key
    JOIN issue_component on issue.issue_key = issue_component.issue_key
    join main.component c on c.id = issue_component.component_id
where date(worklog.started) = DATE('now')
--and c.name = 'Booking-general'
group by c.name
ORDER BY c.name
;
select issue.issue_key from issue left outer join main.issue_component ic on issue.issue_key = ic.issue_key
where ic.id is null;
;

select *
from (select sum(worklog.time_spent_seconds),
             sum(worklog.time_spent_seconds) / 3600      as hours,
             sum(worklog.time_spent_seconds) % 3600 / 60 as minute
      from worklog
      where date(started) = date('now')) shm;

select * from component;
select * from issue_component where issue_key='KT-1774';

select worklog.author, sum(worklog.time_spent_seconds)
from worklog
group by 1;