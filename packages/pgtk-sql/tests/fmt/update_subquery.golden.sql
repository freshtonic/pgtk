UPDATE employees
SET salary = salary * 1.1, updated_at = now()
WHERE department_id IN (
  SELECT id FROM departments WHERE name = 'Engineering'
)
RETURNING id, salary;
