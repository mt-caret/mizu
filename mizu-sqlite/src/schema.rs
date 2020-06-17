table! {
    clients (identity_id, contact_id) {
        identity_id -> Integer,
        contact_id -> Integer,
        client_data -> Binary,
    }
}

table! {
    contacts (id) {
        id -> Integer,
        public_key -> Binary,
        name -> Text,
        created_at -> Text,
    }
}

table! {
    identities (id) {
        id -> Integer,
        name -> Text,
        x3dh_client -> Binary,
        created_at -> Text,
    }
}

table! {
    messages (id) {
        id -> Integer,
        identity_id -> Integer,
        contact_id -> Integer,
        content -> Binary,
        created_at -> Text,
    }
}

joinable!(clients -> contacts (contact_id));
joinable!(messages -> contacts (contact_id));

allow_tables_to_appear_in_same_query!(clients, contacts, identities, messages,);
