diesel::table! {
    tbl_crons (id) {
        id -> Int4,
        timeout -> Int4,
        interval -> Varchar,
        resolver -> Varchar,
    }
}

diesel::table! {
    tbl_fireant_mention (id) {
        id -> Integer,
        symbol -> Varchar,
        mention -> Integer,
        positive -> Integer,
        negative -> Integer,
        promotion -> Integer,
        created_at -> Timestamp,
    }
}

diesel::table! {
    tbl_tcbs_orders (id) {
        id -> Integer,
        symbol -> Varchar,
        side -> Integer,
        price -> Float,
        volume -> Integer,
        ordered_at -> Timestamp,
    }
}

diesel::table! {
    tbl_processes (id) {
        id -> Int4,
        instance -> Varchar,
        command -> Varchar,
        arguments -> Varchar,
    }
}