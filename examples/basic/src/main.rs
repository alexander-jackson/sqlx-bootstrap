use sqlx_bootstrap::BootstrapConfig;

#[tokio::main]
async fn main() -> sqlx::Result<()> {
    let config = BootstrapConfig::new(
        String::from("root"),
        String::from("password"),
        String::from("postgres"),
        String::from("serviceblue"),
        String::from("someotherpassword"),
        String::from("service"),
        String::from("localhost:5432"),
    );

    config.bootstrap().await?;

    Ok(())
}
