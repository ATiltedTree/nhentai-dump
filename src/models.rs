use super::schema::tags;

#[derive(Queryable)]
pub struct Tag {
    pub id: i32,
    pub ty: super::SqlTagType,
    pub name: String,
}

#[derive(Insertable)]
#[table_name = "tags"]
pub struct NewTag {
    pub id: i32,
    pub ty: super::SqlTagType,
    pub name: String,
}

use super::schema::galleries;

#[derive(Queryable)]
pub struct Gallery {
    pub id: i32,
    pub title_english: Option<String>,
    pub title_japanses: Option<String>,
    pub title_pretty: Option<String>,
    pub date: chrono::NaiveDateTime,
    pub num_pages: i32,
}

#[derive(Insertable)]
#[table_name = "galleries"]
pub struct NewGallery<'a> {
    pub id: i32,
    pub title_english: Option<&'a str>,
    pub title_japanese: Option<&'a str>,
    pub title_pretty: Option<&'a str>,
    pub date: chrono::NaiveDateTime,
    pub num_pages: i32,
}

use super::schema::gallery_tags;

#[derive(Queryable)]
pub struct GalleryTag {
    pub id: Option<i32>,
    pub gallery_id: i32,
    pub tag_id: i32,
}

#[derive(Insertable)]
#[table_name = "gallery_tags"]
pub struct NewGalleryTag {
    pub id: Option<i32>,
    pub gallery_id: i32,
    pub tag_id: i32,
}
