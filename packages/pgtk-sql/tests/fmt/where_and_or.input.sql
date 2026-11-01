-- fmt: max_width=60 indent_width=2
select id from employees where salary > 50000 and department_id = 3 and (active or hired_at > now() - interval '1 year');
