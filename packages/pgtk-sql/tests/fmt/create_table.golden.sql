CREATE TABLE projects (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  owner_id INTEGER REFERENCES employees(id),
  created_at timestamptz NOT NULL DEFAULT now()
);
