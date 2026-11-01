-- fmt: max_width=60 indent_width=2
insert into employees (id, first_name, last_name, salary) values (1, 'Ada', 'Lovelace', 100000), (2, 'Alan', 'Turing', 120000) returning id;
