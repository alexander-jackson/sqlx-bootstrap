use sqlx::{postgres::PgConnectOptions, Connection, PgConnection};

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

impl<'a> BootstrapConfig<'a> {
    pub fn new(
        root: RootConfig<'a>,
        app: ApplicationConfig<'a>,
        conn: ConnectionConfig<'a>,
    ) -> Self {
        Self { root, app, conn }
    }

    async fn bootstrap_user(&self, conn: &mut PgConnection) -> sqlx::Result<()> {
        // Check whether the role already exists
        let existing_role = sqlx::query("SELECT oid FROM pg_roles WHERE rolname = $1")
            .bind(self.app.username)
            .fetch_optional(&mut *conn)
            .await?;

        if existing_role.is_some() {
            println!("Role already exists");
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
            println!("Database already exists");
            return Ok(());
        }

        self.grant_role_to_root(conn).await?;
        self.create_database(conn).await?;
        self.revoke_role_from_root(conn).await?;

        Ok(())
    }

    pub async fn bootstrap(&self) -> sqlx::Result<()> {
        let options = PgConnectOptions::new()
            .host(&self.conn.host)
            .port(self.conn.port)
            .username(&self.root.username)
            .password(&self.root.password)
            .database(&self.root.database);

        let mut conn = PgConnection::connect_with(&options).await?;

        self.bootstrap_user(&mut conn).await?;
        self.bootstrap_database(&mut conn).await?;

        Ok(())
    }
}
