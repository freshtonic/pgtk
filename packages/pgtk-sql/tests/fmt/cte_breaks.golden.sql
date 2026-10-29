WITH recent AS (
  SELECT *
  FROM orders
  WHERE created_at > now() - INTERVAL '7 days'
),
totals AS (
  SELECT customer_id, sum(amount) AS total
  FROM recent
  GROUP BY customer_id
)
SELECT c.name, t.total
FROM customers c JOIN totals t ON t.customer_id = c.id;
