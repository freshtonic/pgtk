-- fmt: max_width=60 indent_width=2
select case when salary > 100000 then 'high' when salary > 50000 then 'medium' else 'low' end as band, count(*) from employees group by 1;
