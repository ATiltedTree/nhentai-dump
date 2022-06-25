table! {
    galleries (id) {
        id -> Integer,
        title_english -> Nullable<Text>,
        title_japanese -> Nullable<Text>,
        title_pretty -> Nullable<Text>,
        date -> Timestamp,
        num_pages -> Integer,
    }
}

table! {
    gallery_tags (id) {
        id -> Nullable<Integer>,
        gallery_id -> Integer,
        tag_id -> Integer,
    }
}

table! {
    tags (id) {
        id -> Integer,
        ty -> Integer,
        name -> Text,
    }
}

joinable!(gallery_tags -> galleries (gallery_id));
joinable!(gallery_tags -> tags (tag_id));

allow_tables_to_appear_in_same_query!(galleries, gallery_tags, tags,);
