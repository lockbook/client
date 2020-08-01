use std::marker::PhantomData;

use sled::Db;

use crate::client;
use crate::client::Client;
use crate::model::account::Account;
use crate::model::api::NewAccountError;
use crate::model::crypto::SignedValue;
use crate::repo::account_repo;
use crate::repo::account_repo::AccountRepo;
use crate::repo::file_metadata_repo;
use crate::repo::file_metadata_repo::FileMetadataRepo;
use crate::service::account_service::AccountCreationError::{
    AccountExistsAlready, AccountRepoDbError,
};
use crate::service::auth_service::{AuthGenError, AuthService};
use crate::service::crypto_service::PubKeyCryptoService;
use crate::service::file_encryption_service::{FileEncryptionService, RootFolderCreationError};

#[derive(Debug)]
pub enum AccountCreationError {
    KeyGenerationError(rsa::errors::Error),
    AccountRepoError(account_repo::AccountRepoError),
    AccountRepoDbError(account_repo::DbError),
    FolderError(RootFolderCreationError),
    MetadataRepoError(file_metadata_repo::DbError),
    ApiError(client::Error<NewAccountError>),
    KeySerializationError(serde_json::error::Error),
    AuthGenFailure(AuthGenError),
    AccountExistsAlready,
}

#[derive(Debug)]
pub enum AccountImportError {
    AccountStringCorrupted(base64::DecodeError),
    AccountStringFailedToDeserialize(bincode::Error),
    PersistenceError(account_repo::AccountRepoError),
    InvalidPrivateKey(rsa::errors::Error),
    AccountRepoDbError(account_repo::DbError),
    AccountExistsAlready,
}

#[derive(Debug)]
pub enum AccountExportError {
    AccountRetrievalError(account_repo::AccountRepoError),
    AccountStringFailedToSerialize(bincode::Error),
}

pub trait AccountService {
    fn create_account(db: &Db, username: &str) -> Result<Account, AccountCreationError>;
    fn import_account(db: &Db, account_string: &str) -> Result<Account, AccountImportError>;
    fn export_account(db: &Db) -> Result<String, AccountExportError>;
}

pub struct AccountServiceImpl<
    Crypto: PubKeyCryptoService,
    AccountDb: AccountRepo,
    ApiClient: Client,
    Auth: AuthService,
    FileCrypto: FileEncryptionService,
    FileMetadata: FileMetadataRepo,
> {
    encryption: PhantomData<Crypto>,
    accounts: PhantomData<AccountDb>,
    client: PhantomData<ApiClient>,
    auth: PhantomData<Auth>,
    file_crypto: PhantomData<FileCrypto>,
    file_db: PhantomData<FileMetadata>,
}

impl<
        Crypto: PubKeyCryptoService,
        AccountDb: AccountRepo,
        ApiClient: Client,
        Auth: AuthService,
        FileCrypto: FileEncryptionService,
        FileMetadata: FileMetadataRepo,
    > AccountService
    for AccountServiceImpl<Crypto, AccountDb, ApiClient, Auth, FileCrypto, FileMetadata>
{
    fn create_account(db: &Db, username: &str) -> Result<Account, AccountCreationError> {
        info!("Checking if account already exists");
        if AccountDb::maybe_get_account(&db)
            .map_err(AccountRepoDbError)?
            .is_some()
        {
            return Err(AccountExistsAlready);
        }

        info!("Creating new account for {}", username);

        info!("Generating Key...");
        let keys = Crypto::generate_key().map_err(AccountCreationError::KeyGenerationError)?;

        let account = Account {
            username: String::from(username),
            keys,
        };

        info!("Generating Root Folder");
        let mut file_metadata = FileCrypto::create_metadata_for_root_folder(&account)
            .map_err(AccountCreationError::FolderError)?;

        info!("Sending username & public key to server");
        let auth = SignedValue {
            content: "".to_string(),
            signature: "".to_string(),
        };

        let version = ApiClient::new_account(
            &account.username,
            &auth,
            account.keys.to_public_key(),
            file_metadata.id,
            file_metadata.folder_access_keys.clone(),
            file_metadata
                .user_access_keys
                .get(&account.username)
                .unwrap()
                .access_key
                .clone(),
        )
        .map_err(AccountCreationError::ApiError)?;
        info!("Account creation success!");

        file_metadata.metadata_version = version;
        file_metadata.content_version = version;

        FileMetadata::insert(&db, &file_metadata)
            .map_err(AccountCreationError::MetadataRepoError)?;

        debug!(
            "{}",
            serde_json::to_string(&account).map_err(AccountCreationError::KeySerializationError)?
        );

        info!("Saving account locally");
        AccountDb::insert_account(db, &account).map_err(AccountCreationError::AccountRepoError)?;

        Ok(account)
    }

    fn import_account(db: &Db, account_string: &str) -> Result<Account, AccountImportError> {
        info!("Checking if account already exists");
        if AccountDb::maybe_get_account(&db)
            .map_err(AccountImportError::AccountRepoDbError)?
            .is_some()
        {
            return Err(AccountImportError::AccountExistsAlready);
        }

        info!("Importing account string: {}", &account_string);

        let decoded =
            base64::decode(&account_string).map_err(AccountImportError::AccountStringCorrupted)?;
        debug!("Key is valid base64 string");

        let account: Account = bincode::deserialize(&decoded[..])
            .map_err(AccountImportError::AccountStringFailedToDeserialize)?;
        debug!("Key was valid bincode");

        account
            .keys
            .validate()
            .map_err(AccountImportError::InvalidPrivateKey)?;
        debug!("RSA says the key is valid");

        info!("Account String seems valid, saving now");
        AccountDb::insert_account(db, &account).map_err(AccountImportError::PersistenceError)?;

        // TODO fetch root folder? Kick off sync

        info!("Account imported successfully");
        Ok(account)
    }

    fn export_account(db: &Db) -> Result<String, AccountExportError> {
        let account =
            &AccountDb::get_account(&db).map_err(AccountExportError::AccountRetrievalError)?;
        let encoded: Vec<u8> = bincode::serialize(&account)
            .map_err(AccountExportError::AccountStringFailedToSerialize)?;
        Ok(base64::encode(&encoded))
    }
}
