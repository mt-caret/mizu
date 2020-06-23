table! {
    clients (identity_id, contact_id) {
        identity_id -> Integer,
        contact_id -> Integer,
        client_data -> Binary,
        latest_message_timestamp -> Nullable<Timestamp>,
    }
}

table! {
    contacts (id) {
        id -> Integer,
        address -> Text,
        name -> Text,
        created_at -> Timestamp,
    }
}

table! {
    identities (id) {
        id -> Integer,
        name -> Text,
        x3dh_client -> Binary,
        created_at -> Timestamp,
    }
}

table! {
    messages (id) {
        id -> Integer,
        identity_id -> Integer,
        contact_id -> Integer,
        content -> Binary,
        created_at -> Timestamp,
    }
}

joinable!(clients -> contacts (contact_id));
joinable!(clients -> identities (identity_id));
joinable!(messages -> contacts (contact_id));
joinable!(messages -> identities (identity_id));

allow_tables_to_appear_in_same_query!(clients, contacts, identities, messages,);
