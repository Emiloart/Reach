use sqlx::{migrate::Migrator, postgres::PgPoolOptions, Executor, PgPool};
use std::{
    env,
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};
use tempfile::TempDir;
use thiserror::Error;
use tokio::time::sleep;
use uuid::Uuid;

pub struct CockroachTestContext {
    _temp_dir: TempDir,
    process: Child,
    admin_database_url: String,
}

impl CockroachTestContext {
    pub async fn start() -> Result<Self, TestSupportError> {
        let binary_path = find_cockroach_binary()?;
        let sql_port = pick_unused_port()?;
        let http_port = pick_unused_port()?;
        let temp_dir = TempDir::new().map_err(TestSupportError::TempDir)?;

        let mut process = Command::new(binary_path)
            .args([
                "start-single-node",
                "--insecure",
                &format!("--listen-addr=127.0.0.1:{sql_port}"),
                &format!("--http-addr=127.0.0.1:{http_port}"),
                &format!("--store={}", temp_dir.path().display()),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(TestSupportError::ProcessSpawn)?;

        let admin_database_url =
            format!("postgresql://root@127.0.0.1:{sql_port}/defaultdb?sslmode=disable");

        wait_for_database(&admin_database_url).await?;

        if let Some(status) = process
            .try_wait()
            .map_err(TestSupportError::ProcessStatus)?
        {
            return Err(TestSupportError::ProcessExited(status.code()));
        }

        Ok(Self {
            _temp_dir: temp_dir,
            process,
            admin_database_url,
        })
    }

    pub async fn provision_database(
        &self,
        prefix: &str,
        migrator: &'static Migrator,
    ) -> Result<PgPool, TestSupportError> {
        let database_name = format!(
            "{}_{}",
            sanitize_identifier(prefix),
            Uuid::now_v7().simple()
        );
        let admin_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&self.admin_database_url)
            .await
            .map_err(TestSupportError::DatabaseConnect)?;

        admin_pool
            .execute(format!("CREATE DATABASE {database_name}").as_str())
            .await
            .map_err(TestSupportError::DatabaseCreate)?;

        let database_url = self
            .admin_database_url
            .replace("/defaultdb?", &format!("/{database_name}?"));

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .map_err(TestSupportError::DatabaseConnect)?;

        let cockroach_migrator = Migrator {
            migrations: migrator.migrations.clone(),
            ignore_missing: migrator.ignore_missing,
            locking: false,
            no_tx: migrator.no_tx,
        };

        cockroach_migrator
            .run(&pool)
            .await
            .map_err(TestSupportError::Migrate)?;

        Ok(pool)
    }
}

impl Drop for CockroachTestContext {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

async fn wait_for_database(database_url: &str) -> Result<(), TestSupportError> {
    for _ in 0..60 {
        match PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(1))
            .max_connections(1)
            .connect(database_url)
            .await
        {
            Ok(pool) => {
                pool.close().await;
                return Ok(());
            }
            Err(_) => sleep(Duration::from_millis(500)).await,
        }
    }

    Err(TestSupportError::StartupTimeout)
}

fn find_cockroach_binary() -> Result<PathBuf, TestSupportError> {
    let mut candidates = Vec::new();

    if let Some(bin) = env::var_os("REACH_COCKROACH_BIN") {
        candidates.push(PathBuf::from(bin));
    }

    if let Some(repo_root) = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
    {
        candidates.push(repo_root.join(".tools/cockroach/cockroach.exe"));
        candidates.push(repo_root.join(".tools/cockroach/cockroach"));
    }

    candidates.push(PathBuf::from(".tools/cockroach/cockroach.exe"));
    candidates.push(PathBuf::from(".tools/cockroach/cockroach"));
    candidates.push(PathBuf::from("cockroach"));

    candidates
        .into_iter()
        .find(|path| {
            Command::new(path)
                .arg("version")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        })
        .ok_or(TestSupportError::CockroachBinaryNotFound)
}

fn pick_unused_port() -> Result<u16, TestSupportError> {
    TcpListener::bind("127.0.0.1:0")
        .map_err(TestSupportError::PortBind)?
        .local_addr()
        .map_err(TestSupportError::LocalAddr)
        .map(|address| address.port())
}

fn sanitize_identifier(prefix: &str) -> String {
    let sanitized: String = prefix
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => character,
            _ => '_',
        })
        .collect();

    sanitized.trim_matches('_').to_lowercase()
}

#[derive(Debug, Error)]
pub enum TestSupportError {
    #[error(
        "cockroach binary was not found; set REACH_COCKROACH_BIN or place it in .tools/cockroach"
    )]
    CockroachBinaryNotFound,
    #[error("failed to create temporary store directory: {0}")]
    TempDir(std::io::Error),
    #[error("failed to pick an unused local port: {0}")]
    PortBind(std::io::Error),
    #[error("failed to inspect chosen local address: {0}")]
    LocalAddr(std::io::Error),
    #[error("failed to start cockroach process: {0}")]
    ProcessSpawn(std::io::Error),
    #[error("failed to inspect cockroach process status: {0}")]
    ProcessStatus(std::io::Error),
    #[error("cockroach process exited before startup completed: {0:?}")]
    ProcessExited(Option<i32>),
    #[error("timed out waiting for cockroach to accept SQL connections")]
    StartupTimeout,
    #[error("failed to connect to cockroach: {0}")]
    DatabaseConnect(sqlx::Error),
    #[error("failed to create test database: {0}")]
    DatabaseCreate(sqlx::Error),
    #[error("failed to apply migrations: {0}")]
    Migrate(sqlx::migrate::MigrateError),
}
