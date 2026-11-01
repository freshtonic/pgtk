SELECT id
FROM employees
WHERE salary > 50000
  AND department_id = 3
  AND (active OR hired_at > now() - INTERVAL '1 year');
