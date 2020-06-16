CREATE TABLE contacts(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    public_key BLOB NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE identities(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL,
    x3dh_client BLOB NOT NULL, -- mizu_crypto::x3dh::X3DHClient in bincode
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE clients(
    identity_id INTEGER NOT NULL,
    contact_id INTEGER NOT NULL,
    client_data BLOB NOT NULL, -- mizu_crypto::Client in bincode
    PRIMARY KEY(identity_id, contact_id),
    FOREIGN KEY(identity_id) REFERENCES identity_id(id),
    FOREIGN KEY(contact_id) REFERENCES contacts(id)
);

CREATE TABLE messages(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    identity_id INTEGER NOT NULL,
    contact_id INTEGER NOT NULL,
    content BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(identity_id) REFERENCES identity_id(id),
    FOREIGN KEY(contact_id) REFERENCES contacts(id)
);
