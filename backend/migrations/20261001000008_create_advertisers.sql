CREATE TABLE advertisers (
    id         bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name       text NOT NULL CHECK (name <> ''),
    created_at timestamptz NOT NULL DEFAULT now()
);
