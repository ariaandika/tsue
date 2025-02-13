-- Add up migration script here
create table orders (
  order_id serial,
  name text not null,
  created_at timestamptz default current_timestamp
);

alter table orders add primary key (order_id);

