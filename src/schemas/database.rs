
diesel::table! {
    tbl_users (id) {
        id -> Int4,
        username -> Varchar,
        password -> Varchar,
    }
}

diesel::table! {
    tbl_fireant_mention (id) {
        id -> Integer,
        symbol -> Varchar,
        mention -> Integer,
        positive -> Integer,
        negative -> Integer,
        created_at -> Timestamp,
    }
}

