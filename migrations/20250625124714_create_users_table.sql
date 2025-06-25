-- Add migration script here
CREATE TABLE
    IF NOT EXISTS "users" (
        "id" SERIAL PRIMARY KEY,
        "firstname" VARCHAR(100) NOT NULL,
        "lastname" VARCHAR(100) NOT NULL,
        "email" VARCHAR(255) NOT NULL UNIQUE,
        "password" VARCHAR(100) NOT NULL,
        "created_at" TIMESTAMP
        WITH
            TIME ZONE DEFAULT NOW(),
            "updated_at" TIMESTAMP
        WITH TIME ZONE DEFAULT NOW()
    );

CREATE INDEX IF NOT EXISTS users_email_idx ON users (email);