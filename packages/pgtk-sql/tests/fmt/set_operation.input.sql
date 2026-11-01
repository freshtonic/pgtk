-- fmt: max_width=60 indent_width=2
select id, name from current_employees union all select id, name from former_employees order by name;
