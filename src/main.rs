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
    let client = reqwest::Client::new();

    let start: i32 = galleries
        .select(id)
        .order(id.desc())
        .limit(1)
        .get_results(&db)
        .unwrap()[0];

    let start = start as u32 + 1;

    let mut missing = 0;

    'id: for i in start.. {
        'rep: loop {
            let res = process(&client, &db, i).await;

            match res {
                Ok(_) => {
                    println!("{i}");
                    missing = 0;
                    break 'rep;
                }
                Err(err)
                    if err
                        .downcast_ref::<std::io::Error>()
                        .map(|x| x.kind() == ErrorKind::NotFound)
                        .unwrap_or(false) =>
                {
                    if missing == 10 {
                        break 'id;
                    }
                    missing += 1;
                    break 'rep;
                }
                // retry
                Err(_) => {}
            }
        }
    }

    Ok(())
}

async fn process(
    client: &reqwest::Client,
    db: &SqliteConnection,
    id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let res: Result<models::Gallery, diesel::result::Error> = schema::galleries::dsl::galleries
        .find(id as i32)
        .get_result(db);

    if res.is_ok() {
        return Ok(());
    }

    let hentai = graphql_client::reqwest::post_graphql::<queries::Id, _>(
        client,
        "https://api.hifumin.app",
        queries::id::Variables { id: id as i64 },
    )
    .await?
    .data
    .unwrap()
    .nhentai
    .by;

    let id = if let Some(id) = hentai.id {
        id
    } else {
        return Err(std::io::Error::new(ErrorKind::NotFound, "Not Found").into());
    };

    let new_gallery = models::NewGallery {
        id: id as i32,
        title_english: hentai.title.english.as_deref(),
        title_japanese: hentai.title.japanese.as_deref(),
        title_pretty: hentai.title.pretty.as_deref(),
        date: NaiveDateTime::from_timestamp(hentai.upload_date.unwrap(), 0),
        num_pages: hentai.num_pages.unwrap() as i32,
    };

    diesel::insert_into(schema::galleries::table)
        .values(&new_gallery)
        .execute(db)
        .expect("new gallery");

    let mut new_tags = vec![];
    let mut new_gallery_tags = vec![];

    for tag in hentai.tags {
        let res: Result<models::Tag, diesel::result::Error> =
            schema::tags::dsl::tags.find(tag.id as i32).get_result(db);
        if res.is_err() {
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

    diesel::insert_into(schema::tags::table)
        .values(&new_tags)
        .execute(db)
        .expect("new tag");

    diesel::insert_into(schema::gallery_tags::table)
        .values(&new_gallery_tags)
        .execute(db)
        .expect("new gallery_tag");

    Ok(())
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
