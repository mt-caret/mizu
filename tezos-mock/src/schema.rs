table! {
    messages (id) {
        id -> Integer,
        user_id -> Integer,
        content -> Binary,
        timestamp -> Text,
    }
}

table! {
    pokes (id) {
        id -> Integer,
        user_id -> Integer,
        content -> Binary,
    }
}

table! {
    users (id) {
        id -> Integer,
        address -> Binary,
        identity_key -> Binary,
        prekey -> Binary,
    }
}

joinable!(messages -> users (user_id));
joinable!(pokes -> users (user_id));

allow_tables_to_appear_in_same_query!(messages, pokes, users,);
