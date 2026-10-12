CREATE TABLE IF NOT EXISTS books (
    id          SERIAL PRIMARY KEY,
    title       TEXT NOT NULL,
    author      TEXT NOT NULL,
    price_cents INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS orders (
    id            SERIAL PRIMARY KEY,
    book_id       INTEGER NOT NULL REFERENCES books(id),
    customer_name TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO books (title, author, price_cents)
SELECT * FROM (VALUES
    ('The Rust Programming Language', 'Steve Klabnik & Carol Nichols', 3999),
    ('Designing Data-Intensive Applications', 'Martin Kleppmann', 4999),
    ('The Phoenix Project', 'Gene Kim', 1999),
    ('Site Reliability Engineering', 'Betsy Beyer et al.', 4500),
    ('Clean Code', 'Robert C. Martin', 3500),
    ('The Pragmatic Programmer', 'Andrew Hunt & David Thomas', 3800)
) AS v(title, author, price_cents)
WHERE NOT EXISTS (SELECT 1 FROM books);
