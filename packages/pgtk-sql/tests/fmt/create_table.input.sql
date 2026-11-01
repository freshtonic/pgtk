-- fmt: max_width=60 indent_width=2
create table projects (id serial primary key, name text not null, owner_id integer references employees (id), created_at timestamptz not null default now());
