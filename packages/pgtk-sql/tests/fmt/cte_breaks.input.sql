-- fmt: max_width=60 indent_width=2
with recent as (select * from orders where created_at > now() - interval '7 days'), totals as (select customer_id, sum(amount) as total from recent group by customer_id) select c.name, t.total from customers c join totals t on t.customer_id = c.id;
