SELECT id, name
FROM current_employees
UNION ALL
SELECT id, name
FROM former_employees
ORDER BY name;
