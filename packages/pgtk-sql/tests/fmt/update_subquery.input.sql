-- fmt: max_width=60 indent_width=2
update employees set salary = salary * 1.1, updated_at = now() where department_id in (select id from departments where name = 'Engineering') returning id, salary;
