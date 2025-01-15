use std::fmt;

use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, PgConnection, PgPool};

#[derive(Clone, Debug)]
pub struct RootConfig<'a> {
    username: &'a str,
    password: &'a str,
    database: &'a str,
}

impl<'a> RootConfig<'a> {
    pub fn new(username: &'a str, password: &'a str, database: &'a str) -> Self {
        Self {
            username,
            password,
            database,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ApplicationConfig<'a> {
    username: &'a str,
    password: &'a str,
    database: &'a str,
}

impl<'a> ApplicationConfig<'a> {
    pub fn new(username: &'a str, password: &'a str, database: &'a str) -> Self {
        Self {
            username,
            password,
            database,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionConfig<'a> {
    host: &'a str,
    port: u16,
}

impl<'a> ConnectionConfig<'a> {
    pub fn new(host: &'a str, port: u16) -> Self {
        Self { host, port }
    }
}

#[derive(Clone, Debug)]
pub struct BootstrapConfig<'a> {
    root: RootConfig<'a>,
    app: ApplicationConfig<'a>,
    conn: ConnectionConfig<'a>,
}

#[derive(Debug)]
pub enum BootstrapFromEnvironmentError {
    VarError(std::env::VarError),
    ParseIntError(std::num::ParseIntError),
    SqlxError(sqlx::Error),
}

impl From<std::env::VarError> for BootstrapFromEnvironmentError {
    fn from(value: std::env::VarError) -> Self {
        Self::VarError(value)
    }
}

impl From<std::num::ParseIntError> for BootstrapFromEnvironmentError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::ParseIntError(value)
    }
}

impl From<sqlx::Error> for BootstrapFromEnvironmentError {
    fn from(value: sqlx::Error) -> Self {
        Self::SqlxError(value)
    }
}

impl fmt::Display for BootstrapFromEnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VarError(value) => value.fmt(f),
            Self::ParseIntError(value) => value.fmt(f),
            Self::SqlxError(value) => value.fmt(f),
        }
    }
}

impl std::error::Error for BootstrapFromEnvironmentError {}

impl<'a> BootstrapConfig<'a> {
    pub fn new(
        root: RootConfig<'a>,
        app: ApplicationConfig<'a>,
        conn: ConnectionConfig<'a>,
    ) -> Self {
        Self { root, app, conn }
    }

    pub async fn bootstrap_from_env() -> Result<PgPool, BootstrapFromEnvironmentError> {
        let root_username = std::env::var("ROOT_USERNAME")?;
        let root_password = std::env::var("ROOT_PASSWORD")?;
        let root_database = std::env::var("ROOT_DATABASE")?;

        let app_username = std::env::var("APP_USERNAME")?;
        let app_password = std::env::var("APP_PASSWORD")?;
        let app_database = std::env::var("APP_DATABASE")?;

        let host = std::env::var("DATABASE_HOST")?;
        let port = std::env::var("DATABASE_PORT")?.parse()?;

        let root_config = RootConfig::new(&root_username, &root_password, &root_database);
        let app_config = ApplicationConfig::new(&app_username, &app_password, &app_database);
        let conn_config = ConnectionConfig::new(&host, port);

        let config = BootstrapConfig::new(root_config, app_config, conn_config);
        let pool = config.bootstrap().await?;

        Ok(pool)
    }

    async fn bootstrap_user(&self, conn: &mut PgConnection) -> sqlx::Result<()> {
        // Check whether the role already exists
        let existing_role = sqlx::query("SELECT oid FROM pg_roles WHERE rolname = $1")
            .bind(self.app.username)
            .fetch_optional(&mut *conn)
            .await?;

        if existing_role.is_some() {
            tracing::info!(
                "Role '{}' already exists on the instance",
                self.app.username
            );

            return Ok(());
        }

        // `sqlx` doesn't seem to like doing this in `query!`
        let query = format!(
            "CREATE USER {} PASSWORD '{}'",
            self.app.username, self.app.password
        );

        sqlx::query(&query).execute(conn).await?;

        Ok(())
    }

    async fn grant_role_to_root(&self, conn: &mut PgConnection) -> sqlx::Result<()> {
        let query = format!("GRANT {} TO {}", self.app.username, self.root.username);

        sqlx::query(&query).execute(conn).await?;

        Ok(())
    }

    async fn create_database(&self, conn: &mut PgConnection) -> sqlx::Result<()> {
        let query = format!(
            "CREATE DATABASE {} OWNER {}",
            self.app.database, self.app.username
        );

        sqlx::query(&query).execute(conn).await?;

        tracing::info!("Created database '{}' on the host", self.app.database);

        Ok(())
    }

    async fn revoke_role_from_root(&self, conn: &mut PgConnection) -> sqlx::Result<()> {
        let query = format!("REVOKE {} FROM {}", self.app.username, self.root.username);

        sqlx::query(&query).execute(conn).await?;

        Ok(())
    }

    async fn bootstrap_database(&self, conn: &mut PgConnection) -> sqlx::Result<()> {
        // Check whether the database already exists
        let existing_database = sqlx::query("SELECT oid FROM pg_database WHERE datname = $1")
            .bind(self.app.database)
            .fetch_optional(&mut *conn)
            .await?;

        if existing_database.is_some() {
            tracing::info!(
                "Database '{}' already exists, not creating anything",
                self.app.database
            );

            return Ok(());
        }

        self.grant_role_to_root(conn).await?;
        self.create_database(conn).await?;
        self.revoke_role_from_root(conn).await?;

        Ok(())
    }

    pub async fn bootstrap(&self) -> sqlx::Result<PgPool> {
        let options = PgConnectOptions::new()
            .host(self.conn.host)
            .port(self.conn.port)
            .username(self.root.username)
            .password(self.root.password)
            .database(self.root.database);

        let mut conn = PgConnection::connect_with(&options).await?;

        // Bootstrap the user and database
        self.bootstrap_user(&mut conn).await?;
        self.bootstrap_database(&mut conn).await?;

        // Connect to the application database
        let connection_options = PgConnectOptions::new()
            .host(self.conn.host)
            .port(self.conn.port)
            .username(self.app.username)
            .password(self.app.password)
            .database(self.app.database);

        let pool = PgPool::connect_with(connection_options).await?;

        Ok(pool)
    }
}
