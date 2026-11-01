-- fmt: max_width=60 indent_width=2
select x.n, row_number() over (partition by x.dept order by x.salary desc) as rn from (select dept, salary, name as n from employees) x where x.rn <= 3;
