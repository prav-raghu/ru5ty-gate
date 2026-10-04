use sqlx::Postgres;
use sqlx::query_builder::Separated;

use crate::models::Venue;
use crate::repositories::Writable;

#[derive(Debug, Clone)]
pub struct NewVenue {
    pub code: String,
    pub name: String,
    pub session_duration_secs: i32,
    pub redirect_url: Option<String>,
    pub allow_new_sessions: bool,
    pub created_by: String,
}

impl Writable for NewVenue {
    type Entity = Venue;
    const COLUMNS: &'static [&'static str] = &[
        "code",
        "name",
        "session_duration_secs",
        "redirect_url",
        "allow_new_sessions",
        "created_by",
        "modified_by",
    ];

    fn bind_values(&self, values: &mut Separated<'_, Postgres, &'static str>) {
        values
            .push_bind(&self.code)
            .push_bind(&self.name)
            .push_bind(self.session_duration_secs)
            .push_bind(&self.redirect_url)
            .push_bind(self.allow_new_sessions)
            .push_bind(&self.created_by)
            .push_bind(&self.created_by);
    }
}
