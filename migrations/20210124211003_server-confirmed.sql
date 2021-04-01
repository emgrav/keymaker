-- Add migration script here
CREATE TABLE IF NOT EXISTS servers_backup AS
SELECT *
FROM servers;

DROP TABLE servers cascade;

CREATE TABLE servers
(
    name                text,
    url                 text PRIMARY KEY,
    server_name         text,   -- The part in a mxid to the right of the colons
    logo_url            text,
    admins              text[],
    categories          text[], -- Dont link tables to prevent infinite loop!
    rules               text,
    description         text,
    registration_status registration,
    verified            boolean NOT NULL
);

INSERT INTO servers(name,url, server_name, logo_url,admins,categories,rules,description,registration_status, verified)
SELECT name,url, server_name, logo_url,admins,categories,rules,description,registration_status, true
FROM servers_backup;
