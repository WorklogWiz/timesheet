SELECT component.name                              AS component_name,
       sum(worklog.time_spent_seconds) / 3600      as hours,
       SUM(worklog.time_spent_seconds) % 3600 / 60 AS minutes
FROM issue
    JOIN
    worklog ON issue.ISSUE_KEY = worklog.ISSUE_KEY
    left outer join
    issue_component ON issue.ISSUE_KEY = issue_component.ISSUE_KEY
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
where ISSUE_KEY in ('KT-1892', 'KT-2759')
  and date(worklog.started) = date('now');

SELECT  c.name,
        sum(time_spent_seconds),
        sum(time_spent_seconds) / 3600 as hours,
        sum(time_spent_seconds / 60) % 60 as minutes

FROM main.issue
    join worklog ON issue.ISSUE_KEY = worklog.ISSUE_KEY
    JOIN issue_component on issue.ISSUE_KEY = issue_component.ISSUE_KEY
    join main.component c on c.id = issue_component.component_id
where date(worklog.started) = DATE('now')
--and c.name = 'Booking-general'
group by c.name
ORDER BY c.name
;
select issue.ISSUE_KEY from issue left outer join main.issue_component ic on issue.ISSUE_KEY = ic.ISSUE_KEY
where ic.id is null;
;

select *
from (select sum(worklog.time_spent_seconds),
             sum(worklog.time_spent_seconds) / 3600      as hours,
             sum(worklog.time_spent_seconds) % 3600 / 60 as minute
      from worklog
      where date(started) = date('now')) shm;

select * from component;
select * from issue_component where ISSUE_KEY='KT-1774';

select worklog.author, sum(worklog.time_spent_seconds)
from worklog
group by 1;