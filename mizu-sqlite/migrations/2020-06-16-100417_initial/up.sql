CREATE TABLE contacts(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    address TEXT NOT NULL, -- Tezos address in "tz..." format
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Mizu suppports multiple identities, each corresponding to a Tezos addresses
-- keypair. Each Mizu identity has an associated identity keypair and a
-- prekey keypair (via mizu_crypto::x3dh::X3DHClient).
CREATE TABLE identities(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL,
    address TEXT NOT NULL, -- Tezos address
    x3dh_client BLOB NOT NULL, -- mizu_crypto::x3dh::X3DHClient in bincode
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Mizu keeps a mizu_crypto::Client for each (account, contact) pair.
CREATE TABLE clients(
    identity_id INTEGER NOT NULL,
    contact_id INTEGER NOT NULL,
    client_data BLOB NOT NULL, -- mizu_crypto::Client in bincode
    -- the timestamp of the latest message this client processed
    latest_message_timestamp TIMESTAMP,
    PRIMARY KEY(identity_id, contact_id),
    FOREIGN KEY(identity_id) REFERENCES identities(id),
    FOREIGN KEY(contact_id) REFERENCES contacts(id)
);

CREATE TABLE messages(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    identity_id INTEGER NOT NULL,
    contact_id INTEGER NOT NULL,
    content BLOB NOT NULL,
    my_message BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(identity_id) REFERENCES identities(id),
    FOREIGN KEY(contact_id) REFERENCES contacts(id)
);
