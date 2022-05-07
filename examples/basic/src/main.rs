use sqlx_bootstrap::BootstrapConfig;

#[tokio::main]
async fn main() -> sqlx::Result<()> {
    let config = BootstrapConfig::new(
        "root",
        "password",
        "postgres",
        "serviceblue",
        "someotherpassword",
        "service",
        "localhost",
        5432,
    );

    config.bootstrap().await?;

    Ok(())
}
