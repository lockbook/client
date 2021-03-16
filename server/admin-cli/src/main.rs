extern crate structopt;

use structopt::StructOpt;
use postgres::NoTls;
use postgres::Client as PostgresClient;
use postgres::Config as PostgresConfig;

pub mod delete_user;
pub mod clear_untracked_files;
pub mod utils;
pub mod error;

#[derive(Debug, PartialEq, StructOpt)]
#[structopt(about = "The admin client for Lockbook.")]
enum LockbookAdminClient {

    /// Delete a user and all their files recursively
    DeleteUser {
        /// account name that will be deleted
        username: String
    },

    /// Deletes files linked to no users
    ClearUntrackedFiles
}

fn main() {
    let args: LockbookAdminClient = LockbookAdminClient::from_args();

    match args {
        LockbookAdminClient::DeleteUser {
            username
        } => {

        }
        LockbookAdminClient::ClearUntrackedFiles => {}
    }
}

pub async fn connect(config: &IndexDbConfig) -> Result<PostgresClient, ConnectError> {
    let mut postgres_config = PostgresConfig::new();
    postgres_config
        .user(&config.user)
        .host(&config.host)
        .password(&config.pass)
        .port(config.port)
        .dbname(&config.db);

    match config.cert.as_str() {
        "" => connect_no_tls(&postgres_config).await,
        cert => connect_with_tls(&postgres_config, &cert).await,
    }
}

async fn connect_no_tls(postgres_config: &PostgresConfig) -> Result<PostgresClient, ConnectError> {
    match postgres_config.connect(NoTls).await {
        Ok((client, connection)) => {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    panic!("connection error: {}", e);
                }
            });
            Ok(client)
        }
        Err(err) => Err(ConnectError::Postgres(err)),
    }
}

async fn connect_with_tls(
    postgres_config: &PostgresConfig,
    cert: &str,
) -> Result<PostgresClient, ConnectError> {
    let mut builder = match SslConnector::builder(SslMethod::tls()) {
        Ok(builder) => builder,
        Err(err) => return Err(ConnectError::OpenSsl(err)),
    };
    builder.set_ca_file(cert).map_err(ConnectError::OpenSsl)?;
    match postgres_config
        .connect(MakeTlsConnector::new(builder.build()))
        .await
    {
        Ok((client, connection)) => {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    panic!("connection error: {}", e);
                }
            });
            Ok(client)
        }
        Err(err) => Err(ConnectError::Postgres(err)),
    }
}
