CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    name text,
    servers integer[] -- Foreign keys on array values are apparently not possible so we just keep the Ids
);