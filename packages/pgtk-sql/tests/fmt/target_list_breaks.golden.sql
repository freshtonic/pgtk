SELECT
  e.id,
  e.first_name,
  e.last_name,
  d.name AS department,
  e.salary
FROM employees e;
