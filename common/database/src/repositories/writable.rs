use sqlx::Postgres;
use sqlx::query_builder::Separated;

use crate::repositories::Entity;

pub trait Writable {
    type Entity: Entity;
    const COLUMNS: &'static [&'static str];

    fn bind_values(&self, values: &mut Separated<'_, Postgres, &'static str>);
}
