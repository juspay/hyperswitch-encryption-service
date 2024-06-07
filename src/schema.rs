// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;

    data_key_store (id) {
        id -> Int4,
        #[max_length = 255]
        key_identifier -> Varchar,
        #[max_length = 20]
        data_identifier -> Varchar,
        encryption_key -> Bytea,
        version -> Int4,
        created_at -> Timestamp,
    }
}
