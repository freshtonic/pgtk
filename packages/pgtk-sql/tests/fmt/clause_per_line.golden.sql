SELECT e.id, e.first_name, d.name AS department
FROM employees e
JOIN departments d ON d.id = e.department_id
LEFT JOIN locations l ON l.id = d.location_id
WHERE e.salary > 50000 AND d.name <> 'Sales'
ORDER BY e.salary DESC
LIMIT 10;
