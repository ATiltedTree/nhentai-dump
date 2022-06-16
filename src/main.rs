use std::time::Duration;

#[macro_use]
extern crate diesel;

use diesel::backend::Backend;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use diesel::sqlite::SqliteConnection;
use diesel::types::{FromSql, ToSql};
use indicatif::{ProgressBar, ProgressStyle};
use nhentai::gallery::TagType;

mod models;
mod schema;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv()?;

    let db = SqliteConnection::establish(&std::env::var("DATABASE_URL").unwrap())?;
    let client = nhentai::Client::new(std::env::var("NHENTAI_COOKIE").ok().as_deref());

    let start = std::env::args().nth(1).unwrap().parse()?;
    let end = std::env::args().nth(2).unwrap().parse()?;

    let bar = ProgressBar::new(end as u64);

    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:60.cyan/blue} {pos:>7}/{len:7} {eta_precise}")
            .progress_chars("#>-"),
    );
    bar.set_position(start as u64);

    for id in start..=end {
        loop {
            let res = process(&client, &db, id).await;

            match res {
                Ok(_) => {
                    break;
                }
                Err(nhentai::Error::NotFound(_)) => {
                    break;
                }
                // retry
                Err(_) => {}
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        bar.inc(1);
    }
    bar.finish();

    Ok(())
}

async fn process(
    client: &nhentai::Client,
    db: &SqliteConnection,
    id: u32,
) -> Result<(), nhentai::Error> {
    let res: Result<models::Gallery, diesel::result::Error> = schema::galleries::dsl::galleries
        .find(id as i32)
        .get_result(db);

    if res.is_ok() {
        return Ok(());
    }
    let hentai = client.gallery(id).await?;

    let title = hentai.title();

    let new_gallery = models::NewGallery {
        id: hentai.id() as i32,
        title_english: title.english(),
        title_japanese: title.japanese(),
        title_pretty: title.pretty(),
        date: hentai.date().naive_utc(),
        num_pages: hentai.pages_len() as i32,
    };

    diesel::insert_into(schema::galleries::table)
        .values(&new_gallery)
        .execute(db)
        .expect("new gallery");

    let mut new_tags = vec![];
    let mut new_gallery_tags = vec![];

    for tag in hentai.tags() {
        let res: Result<models::Tag, diesel::result::Error> =
            schema::tags::dsl::tags.find(tag.id() as i32).get_result(db);
        if res.is_err() {
            new_tags.push(models::NewTag {
                id: tag.id() as i32,
                ty: SqlTagType(tag.ty()),
                name: tag.name().to_string(),
            });
        }

        new_gallery_tags.push(models::NewGalleryTag {
            id: None,
            gallery_id: hentai.id() as i32,
            tag_id: tag.id() as i32,
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
pub struct SqlTagType(TagType);

impl<DB> ToSql<Integer, DB> for SqlTagType
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<W: std::io::Write>(
        &self,
        out: &mut diesel::serialize::Output<W, DB>,
    ) -> diesel::serialize::Result {
        let id = match self.0 {
            TagType::Tag => 0,
            TagType::Language => 1,
            TagType::Artist => 2,
            TagType::Group => 3,
            TagType::Category => 4,
            TagType::Parody => 5,
            TagType::Character => 6,
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
            0 => TagType::Tag,
            1 => TagType::Language,
            2 => TagType::Artist,
            3 => TagType::Group,
            4 => TagType::Category,
            5 => TagType::Parody,
            6 => TagType::Character,
            _ => return Err(std::io::Error::new(std::io::ErrorKind::Other, "AA").into()),
        };

        Ok(SqlTagType(ty))
        //.map(|x| Self(TagType::from(x as u8)))
    }
}
