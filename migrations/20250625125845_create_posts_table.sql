-- Add migration script here
CREATE TABLE
    IF NOT EXISTS "posts" (
        "id" SERIAL PRIMARY KEY,
        "title" VARCHAR(200) NOT NULL,
        "body" TEXT NOT NULL,
        "category_id" INT NOT NULL,
        "user_id" INT NOT NULL,
        "user_name" VARCHAR(200) NOT NULL,
        "created_at" TIMESTAMP
        WITH
            TIME ZONE DEFAULT NOW(),
            "updated_at" TIMESTAMP
        WITH
            TIME ZONE DEFAULT NOW(),
            FOREIGN KEY (category_id) REFERENCES categories(id) ON UPDATE CASCADE ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE
    );