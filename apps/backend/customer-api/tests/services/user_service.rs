#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use customer_api::services::UserFilters;
use ru5ty_gate_database::sqlx::{self, PgPool};
use uuid::Uuid;

use crate::common::{RecordingEmailSender, UserFactory, build_application, live_redis};

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn lists_active_users_excluding_the_caller(pool: PgPool) {
    let app = build_application(&pool, RecordingEmailSender::new(true), live_redis().await).await;
    let service = &app.state().services.user;
    let me = UserFactory::new(&pool, "Me Myself").create().await;
    UserFactory::new(&pool, "Friend One").create().await;
    UserFactory::new(&pool, "Friend Two").create().await;
    let inactive = UserFactory::new(&pool, "Gone Away").create().await;
    sqlx::query("UPDATE users SET is_active = FALSE WHERE id = $1")
        .bind(inactive)
        .execute(&pool)
        .await
        .unwrap();

    let users = service
        .get_users(&UserFilters::default(), Some(me))
        .await
        .unwrap();

    let names: Vec<_> = users.iter().map(|user| user.username.as_str()).collect();
    assert_eq!(users.len(), 2);
    assert!(names.contains(&"Friend One") && names.contains(&"Friend Two"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn filters_by_gender_age_and_pages_results(pool: PgPool) {
    let app = build_application(&pool, RecordingEmailSender::new(true), live_redis().await).await;
    let service = &app.state().services.user;
    UserFactory::new(&pool, "Young Female")
        .age(20)
        .gender("female")
        .create()
        .await;
    UserFactory::new(&pool, "Older Female")
        .age(50)
        .gender("female")
        .create()
        .await;
    UserFactory::new(&pool, "Middle Male")
        .age(35)
        .gender("male")
        .create()
        .await;

    let females = service
        .get_users(
            &UserFilters {
                gender: Some("female".to_owned()),
                ..UserFilters::default()
            },
            None,
        )
        .await
        .unwrap();
    let in_range = service
        .get_users(
            &UserFilters {
                min_age: Some(30),
                max_age: Some(40),
                ..UserFilters::default()
            },
            None,
        )
        .await
        .unwrap();
    let paged = service
        .get_users(
            &UserFilters {
                limit: Some(1),
                offset: Some(1),
                ..UserFilters::default()
            },
            None,
        )
        .await
        .unwrap();

    assert_eq!(females.len(), 2);
    assert_eq!(in_range.len(), 1);
    assert_eq!(in_range[0].username, "Middle Male");
    assert_eq!(paged.len(), 1);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn user_lookups_respect_activity_and_role(pool: PgPool) {
    let app = build_application(&pool, RecordingEmailSender::new(true), live_redis().await).await;
    let service = &app.state().services.user;
    let customer = UserFactory::new(&pool, "Plain Customer").create().await;
    let admin = UserFactory::new(&pool, "Plain Admin")
        .role("Super Admin")
        .create()
        .await;

    let by_id = service.get_user_by_id(customer).await.unwrap().unwrap();
    let missing = service.get_user_by_id(Uuid::new_v4()).await.unwrap();
    let authorised_customer = service.get_authorized_user_by_id(customer).await.unwrap();
    let authorised_admin = service.get_authorized_user_by_id(admin).await.unwrap();

    assert_eq!(by_id.username, "Plain Customer");
    assert!(missing.is_none());
    assert_eq!(authorised_customer.unwrap().role_name, "Chat User");
    assert!(authorised_admin.is_none());
}
