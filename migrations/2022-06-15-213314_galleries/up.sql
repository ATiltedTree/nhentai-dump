-- Your SQL goes here
CREATE TABLE galleries (
    id INTEGER NOT NULL PRIMARY KEY,
    title_english VARCHAR,
    title_japanese VARCHAR,
    title_pretty VARCHAR,
    date DATETIME NOT NULL,
    num_pages INTEGER NOT NULL
);

CREATE TABLE gallery_tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    gallery_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    FOREIGN KEY(gallery_id) REFERENCES galleries(id),
    FOREIGN KEY(tag_id) REFERENCES tags(id)
);