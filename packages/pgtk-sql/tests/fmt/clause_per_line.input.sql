-- fmt: max_width=60 indent_width=2
select e.id, e.first_name, d.name as department from employees e join departments d on d.id = e.department_id left join locations l on l.id = d.location_id where e.salary > 50000 and d.name <> 'Sales' order by e.salary desc limit 10;
