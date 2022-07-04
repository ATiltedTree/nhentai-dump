#![feature(iter_next_chunk)]

use std::io::ErrorKind;
use std::str::FromStr;

#[macro_use]
extern crate diesel;

use chrono::NaiveDateTime;
use diesel::backend::Backend;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use diesel::sqlite::SqliteConnection;
use diesel::types::{FromSql, ToSql};

mod models;
mod queries;
mod schema;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use schema::galleries::dsl::*;

    let _ = dotenv::dotenv();

    let db = SqliteConnection::establish(&std::env::var("DATABASE_URL").unwrap())?;
    let client = reqwest::Client::builder()
        .user_agent("nhentai-dump/0.1.0")
        .build()
        .unwrap();

    let start: i32 = galleries
        .select(id)
        .order(id.desc())
        .limit(1)
        .get_results(&db)
        .unwrap()
        .get(0)
        .unwrap_or(&-1)
        .to_owned();

    let start = (start + 1) as u32;

    let mut it = start..;

    while let Ok(i) = it.next_chunk::<25>() {
        let res = process(&client, &db, i).await?;

        if res != 25 {
            break;
        }
    }

    Ok(())
}

async fn process(
    client: &reqwest::Client,
    db: &SqliteConnection,
    ids: [u32; 25],
) -> Result<usize, Box<dyn std::error::Error>> {
    let hentai = graphql_client::reqwest::post_graphql::<queries::Id, _>(
        client,
        "https://api.hifumin.app/v1/graphql",
        queries::id::Variables {
            ids: ids.map(|x| x as i64).to_vec(),
        },
    )
    .await
    .unwrap()
    .data
    .unwrap()
    .nhentai
    .multiple;

    let hentais = if hentai.success {
        hentai
            .data
            .into_iter()
            .filter(|x| x.id.is_some())
            .collect::<Vec<_>>()
    } else {
        return Err(
            std::io::Error::new(ErrorKind::Other, hentai.error.unwrap().to_string()).into(),
        );
    };

    let mut new_galleries = vec![];
    let mut new_tags: Vec<models::NewTag> = vec![];
    let mut new_gallery_tags = vec![];

    let len = hentais.len();

    for hentai in hentais {
        let id = hentai.id.unwrap();
        let res: Result<models::Gallery, diesel::result::Error> = schema::galleries::dsl::galleries
            .find(id as i32)
            .get_result(db);
        if res.is_ok() {
            continue;
        }

        let new_gallery = models::NewGallery {
            id: id as i32,
            title_english: hentai.title.english,
            title_japanese: hentai.title.japanese,
            title_pretty: hentai.title.pretty,
            date: NaiveDateTime::from_timestamp(hentai.upload_date.unwrap(), 0),
            num_pages: hentai.num_pages.unwrap() as i32,
        };

        new_galleries.push(new_gallery);

        for tag in hentai.tags {
            let res: Result<models::Tag, diesel::result::Error> =
                schema::tags::dsl::tags.find(tag.id as i32).get_result(db);
            if res.is_err() && new_tags.iter().find(|x| x.id == tag.id as i32).is_none() {
                new_tags.push(models::NewTag {
                    id: tag.id as i32,
                    ty: tag.type_.parse().unwrap(),
                    name: tag.name,
                });
            }

            new_gallery_tags.push(models::NewGalleryTag {
                id: None,
                gallery_id: id as i32,
                tag_id: tag.id as i32,
            });
        }
        println!("{id}");
    }

    diesel::insert_into(schema::galleries::table)
        .values(&new_galleries)
        .execute(db)
        .expect("new gallery");

    diesel::insert_into(schema::tags::table)
        .values(&new_tags)
        .execute(db)
        .expect("new tag");

    diesel::insert_into(schema::gallery_tags::table)
        .values(&new_gallery_tags)
        .execute(db)
        .expect("new gallery_tag");

    Ok(len)
}

#[derive(Debug, AsExpression, PartialEq, Eq, FromSqlRow)]
#[sql_type = "Integer"]
pub enum SqlTagType {
    Tag,
    Language,
    Artist,
    Group,
    Category,
    Parody,
    Character,
}

impl FromStr for SqlTagType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ty = match s {
            "tag" => Self::Tag,
            "language" => Self::Language,
            "artist" => Self::Artist,
            "group" => Self::Group,
            "category" => Self::Category,
            "parody" => Self::Parody,
            "character" => Self::Character,
            _ => return Err(()),
        };

        Ok(ty)
    }
}

impl<DB> ToSql<Integer, DB> for SqlTagType
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<W: std::io::Write>(
        &self,
        out: &mut diesel::serialize::Output<W, DB>,
    ) -> diesel::serialize::Result {
        let id = match self {
            SqlTagType::Tag => 0,
            SqlTagType::Language => 1,
            SqlTagType::Artist => 2,
            SqlTagType::Group => 3,
            SqlTagType::Category => 4,
            SqlTagType::Parody => 5,
            SqlTagType::Character => 6,
        };

        id.to_sql(out)
    }
}

impl<DB> FromSql<Integer, DB> for SqlTagType
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> diesel::deserialize::Result<Self> {
        let ty = match i32::from_sql(bytes)? {
            0 => SqlTagType::Tag,
            1 => SqlTagType::Language,
            2 => SqlTagType::Artist,
            3 => SqlTagType::Group,
            4 => SqlTagType::Category,
            5 => SqlTagType::Parody,
            6 => SqlTagType::Character,
            _ => return Err(std::io::Error::new(std::io::ErrorKind::Other, "AA").into()),
        };

        Ok(ty)
    }
}
