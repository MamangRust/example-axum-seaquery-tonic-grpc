-- Add migration script here
CREATE TABLE
    IF NOT EXISTS "comments" (
        "id" SERIAL PRIMARY KEY,
        "id_post_comment" INT NOT NULL,
        "user_name_comment" VARCHAR(200) NOT NULL,
        "comment" VARCHAR(200) NOT NULL,
        "created_at" TIMESTAMP
        WITH
            TIME ZONE DEFAULT NOW(),
            "updated_at" TIMESTAMP
        WITH
            TIME ZONE DEFAULT NOW(),
            FOREIGN KEY (id_post_comment) REFERENCES posts(id) ON UPDATE CASCADE ON DELETE CASCADE
    );
