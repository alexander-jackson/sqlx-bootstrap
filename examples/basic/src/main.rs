use sqlx_bootstrap::{ApplicationConfig, BootstrapConfig, ConnectionConfig, RootConfig};

#[tokio::main]
async fn main() -> sqlx::Result<()> {
    let root_config = RootConfig::new("root", "password", "postgres");
    let app_config = ApplicationConfig::new("serviceblue", "somepassword", "service");
    let conn_config = ConnectionConfig::new("localhost", 5432);

    let config = BootstrapConfig::new(root_config, app_config, conn_config);

    config.bootstrap().await?;

    Ok(())
}
