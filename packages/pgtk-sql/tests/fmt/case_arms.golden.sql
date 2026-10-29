SELECT
  CASE
    WHEN salary > 100000 THEN 'high'
    WHEN salary > 50000 THEN 'medium'
    ELSE 'low' END AS band,
  count(*)
FROM employees
GROUP BY 1;
