SELECT
  x.n,
  row_number() OVER (
    PARTITION BY x.dept ORDER BY x.salary DESC
  ) AS rn
FROM (SELECT dept, salary, name AS n FROM employees) x
WHERE x.rn <= 3;
