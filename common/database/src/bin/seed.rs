use ru5ty_gate_config::EnvReader;
use ru5ty_gate_database::{
    DatabaseConfig, DatabaseError, SeedAdmin, connect, seed_admin, seed_roles, seed_user_statuses,
};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Seed failed: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), DatabaseError> {
    let env = EnvReader::from_process();
    let pool = connect(&DatabaseConfig::from_env(&env)?).await?;
    seed_roles(&pool).await?;
    seed_user_statuses(&pool).await?;
    let admin = SeedAdmin {
        email: env.required("ADMIN_EMAIL")?,
        username: env.required("ADMIN_USERNAME")?,
        password: env.required("ADMIN_PASSWORD")?,
    };
    if seed_admin(&pool, &admin).await? {
        println!("Admin user created");
    } else {
        println!("Admin already exists - skipping");
    }
    Ok(())
}
