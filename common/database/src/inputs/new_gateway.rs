use sqlx::Postgres;
use sqlx::query_builder::Separated;
use uuid::Uuid;

use crate::models::Gateway;
use crate::repositories::Writable;

#[derive(Debug, Clone)]
pub struct NewGateway {
    pub id: Uuid,
    pub venue_id: Uuid,
    pub name: String,
    pub api_key_hash: String,
    pub created_by: String,
}

impl Writable for NewGateway {
    type Entity = Gateway;
    const COLUMNS: &'static [&'static str] = &[
        "id",
        "venue_id",
        "name",
        "api_key_hash",
        "created_by",
        "modified_by",
    ];

    fn bind_values(&self, values: &mut Separated<'_, Postgres, &'static str>) {
        values
            .push_bind(self.id)
            .push_bind(self.venue_id)
            .push_bind(&self.name)
            .push_bind(&self.api_key_hash)
            .push_bind(&self.created_by)
            .push_bind(&self.created_by);
    }
}
