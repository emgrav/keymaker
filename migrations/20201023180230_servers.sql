CREATE TYPE registration AS ENUM ('open', 'invite', 'closed');

CREATE TABLE servers (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    url text NOT NULL,
    logo_url text,
    admins text[],
    categories integer[], -- Foreign keys on array values are apparently not possible so we just keep the Ids
    rules text,
    description text,
    registration_status registration
);