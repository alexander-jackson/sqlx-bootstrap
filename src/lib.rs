use sqlx::{Connection, PgConnection};

#[derive(Clone, Debug)]
pub struct BootstrapConfig {
    root_username: String,
    root_password: String,
    root_database_name: String,
    database_username: String,
    database_password: String,
    database_name: String,
    database_uri: String,
}

impl BootstrapConfig {
    pub fn new(
        root_username: String,
        root_password: String,
        root_database_name: String,
        database_username: String,
        database_password: String,
        database_name: String,
        database_uri: String,
    ) -> Self {
        Self {
            root_username,
            root_password,
            root_database_name,
            database_username,
            database_password,
            database_name,
            database_uri,
        }
    }

    async fn bootstrap_user(&self) -> sqlx::Result<()> {
        let url = format!(
            "postgres://{}:{}@{}/{}",
            self.root_username, self.root_password, self.database_uri, self.root_database_name,
        );

        let mut conn = PgConnection::connect(&url).await?;

        // Check whether the role already exists
        let existing_role = sqlx::query!(
            "SELECT oid FROM pg_roles WHERE rolname = $1",
            self.database_username
        )
        .fetch_optional(&mut conn)
        .await?;

        if existing_role.is_some() {
            println!("Role already exists");
            return Ok(());
        }

        // `sqlx` doesn't seem to like doing this in `query!`
        let query = format!(
            "CREATE USER {} PASSWORD '{}'",
            self.database_username, self.database_password
        );

        sqlx::query(&query).execute(&mut conn).await?;

        Ok(())
    }

    async fn grant_role_to_root(&self, conn: &mut PgConnection) -> sqlx::Result<()> {
        let query = format!("GRANT {} TO {}", self.database_username, self.root_username);

        sqlx::query(&query).execute(conn).await?;

        Ok(())
    }

    async fn create_database(&self, conn: &mut PgConnection) -> sqlx::Result<()> {
        let query = format!(
            "CREATE DATABASE {} OWNER {}",
            self.database_name, self.database_username
        );

        sqlx::query(&query).execute(conn).await?;

        Ok(())
    }

    async fn revoke_role_from_root(&self, conn: &mut PgConnection) -> sqlx::Result<()> {
        let query = format!(
            "REVOKE {} FROM {}",
            self.database_username, self.root_username
        );

        sqlx::query(&query).execute(conn).await?;

        Ok(())
    }

    async fn bootstrap_database(&self) -> sqlx::Result<()> {
        let url = format!(
            "postgres://{}:{}@{}/{}",
            self.root_username, self.root_password, self.database_uri, self.root_database_name,
        );

        let mut conn = PgConnection::connect(&url).await?;

        // Check whether the database already exists
        let existing_database = sqlx::query!(
            "SELECT oid FROM pg_database WHERE datname = $1",
            self.database_name
        )
        .fetch_optional(&mut conn)
        .await?;

        if existing_database.is_some() {
            println!("Database already exists");
            return Ok(());
        }

        self.grant_role_to_root(&mut conn).await?;
        self.create_database(&mut conn).await?;
        self.revoke_role_from_root(&mut conn).await?;

        Ok(())
    }

    pub async fn bootstrap(&self) -> sqlx::Result<()> {
        println!("Bootstrapping the database! Config: {:?}", self);

        self.bootstrap_user().await?;
        self.bootstrap_database().await?;

        Ok(())
    }
}
