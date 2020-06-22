CREATE TABLE users(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    address TEXT NOT NULL, -- Tezos address in "tz..." format
    identity_key BLOB NOT NULL,
    prekey BLOB NOT NULL,
    UNIQUE(address)
);

CREATE TABLE messages(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_id INTEGER NOT NULL,
    content BLOB NOT NULL,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE TABLE pokes(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_id INTEGER NOT NULL,
    content BLOB NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id)
);