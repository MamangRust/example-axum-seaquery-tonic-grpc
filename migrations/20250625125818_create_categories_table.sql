-- Add migration script here
CREATE TABLE
    IF NOT EXISTS "categories" (
        "id" SERIAL PRIMARY KEY,
        "name" VARCHAR(200) NOT NULL,
        "created_at" TIMESTAMP
        WITH
            TIME ZONE DEFAULT NOW(),
            "updated_at" TIMESTAMP
        WITH TIME ZONE DEFAULT NOW()
    );