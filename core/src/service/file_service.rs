use sled::Db;

use crate::error_enum;
use crate::model::file_metadata::{FileMetadata, Status};
use crate::repo::account_repo;
use crate::repo::account_repo::AccountRepo;
use crate::repo::file_metadata_repo;
use crate::repo::file_metadata_repo::FileMetadataRepo;
use crate::repo::file_repo;
use crate::repo::file_repo::FileRepo;
use crate::service::crypto_service::DecryptedValue;
use crate::service::file_encryption_service;
use crate::service::file_encryption_service::{EncryptedFile, FileEncryptionService};
use crate::service::logging_service::Logger;
use serde::export::PhantomData;

error_enum! {
    enum NewFileError {
        AccountRetrievalError(account_repo::Error),
        EncryptedFileError(file_encryption_service::FileCreationError),
        SavingMetadataFailed(file_metadata_repo::Error),
        SavingFileContentsFailed(file_repo::Error),
    }
}

error_enum! {
    enum UpdateFileError {
        AccountRetrievalError(account_repo::Error),
        FileRetrievalError(file_repo::Error),
        EncryptedWriteError(file_encryption_service::FileWriteError),
        MetadataDbError(file_metadata_repo::Error),

    }
}

error_enum! {
    enum Error {
        FileRepo(file_repo::Error),
        MetaRepo(file_metadata_repo::Error),
        AccountRepo(account_repo::Error),
        EncryptionServiceWrite(file_encryption_service::FileWriteError),
        EncryptionServiceRead(file_encryption_service::UnableToReadFile),
    }
}

pub trait FileService {
    fn create(db: &Db, name: &String, path: &String) -> Result<FileMetadata, NewFileError>;
    fn update(db: &Db, id: &String, content: &String) -> Result<EncryptedFile, UpdateFileError>;
    fn get(db: &Db, id: &String) -> Result<DecryptedValue, Error>;
}

pub struct FileServiceImpl<
    Log: Logger,
    FileMetadataDb: FileMetadataRepo,
    FileDb: FileRepo,
    AccountDb: AccountRepo,
    FileCrypto: FileEncryptionService,
> {
    log: PhantomData<Log>,
    metadatas: PhantomData<FileMetadataDb>,
    files: PhantomData<FileDb>,
    account: PhantomData<AccountDb>,
    file_crypto: PhantomData<FileCrypto>,
}

impl<
        Log: Logger,
        FileMetadataDb: FileMetadataRepo,
        FileDb: FileRepo,
        AccountDb: AccountRepo,
        FileCrypto: FileEncryptionService,
    > FileService for FileServiceImpl<Log, FileMetadataDb, FileDb, AccountDb, FileCrypto>
{
    fn create(db: &Db, name: &String, path: &String) -> Result<FileMetadata, NewFileError> {
        Log::info(format!(
            "Creating new file with name: {} at path {}",
            name, path
        ));
        let account = AccountDb::get_account(db)?;
        Log::debug(format!("Account retrieved: {:?}", account));

        let encrypted_file = FileCrypto::new_file(&account)?;
        Log::debug(format!("Encrypted file created: {:?}", encrypted_file));

        let meta = FileMetadataDb::insert(&db, &name, &path)?;
        Log::debug(format!("Metadata for file: {:?}", meta));

        FileDb::update(db, &meta.id, &encrypted_file)?;
        Log::info(format!("New file saved locally"));
        Ok(meta)
    }

    fn update(db: &Db, id: &String, content: &String) -> Result<EncryptedFile, UpdateFileError> {
        Log::info(format!(
            "Replacing file id: {} contents with: {}",
            &id, &content
        ));

        let account = AccountDb::get_account(db)?;
        Log::debug(format!("Account retrieved: {:?}", account));

        let encrypted_file = FileDb::get(db, &id)?;
        Log::debug(format!(
            "Metadata of the file to edit: {:?}",
            encrypted_file
        ));

        let updated_enc_file = FileCrypto::write_to_file(
            &account,
            &encrypted_file,
            &DecryptedValue {
                secret: content.clone(),
            },
        )?;
        Log::debug(format!("New encrypted file: {:?}", updated_enc_file));

        FileDb::update(db, &id, &updated_enc_file)?;

        let meta = FileMetadataDb::get(db, &id)?;
        Log::debug(format!("New metadata: {:?}", &meta));
        FileMetadataDb::update(
            db,
            &FileMetadata {
                id: id.clone(),
                name: meta.name,
                path: meta.path,
                updated_at: meta.updated_at,
                version: meta.version,
                status: if meta.status == Status::New {
                    Status::New
                } else {
                    Status::Local
                },
            },
        )?;
        Log::info(format!("Updated file {:?} contents {:?}", &id, &content));
        Ok(updated_enc_file)
    }

    fn get(db: &Db, id: &String) -> Result<DecryptedValue, Error> {
        Log::info(format!("Getting file contents {:?}", &id));
        let account = AccountDb::get_account(db)?;
        let encrypted_file = FileDb::get(db, &id)?;
        let decrypted_file = FileCrypto::read_file(&account, &encrypted_file)?;
        Ok(decrypted_file)
    }
}
