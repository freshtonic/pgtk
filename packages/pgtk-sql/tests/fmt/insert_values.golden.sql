INSERT INTO employees (id, first_name, last_name, salary)
VALUES (1, 'Ada', 'Lovelace', 100000),
  (2, 'Alan', 'Turing', 120000)
RETURNING id;
