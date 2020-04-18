#![feature(try_trait)]
extern crate reqwest;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::Path;

use serde_json::json;
use sled::Db;

use crate::client::ClientImpl;
use crate::crypto::RsaCryptoService;
use crate::model::file::File;
use crate::model::file_metadata::FileMetadata;
use crate::model::state::Config;
use crate::repo::account_repo::{AccountRepo, AccountRepoImpl};
use crate::repo::db_provider::{DbProvider, DiskBackedDB};
use crate::repo::file_metadata_repo::{FileMetadataRepo, FileMetadataRepoImpl};
use crate::repo::file_repo::{FileRepo, FileRepoImpl};
use crate::service::account_service::{AccountService, AccountServiceImpl};
use crate::service::file_metadata_service::{FileMetadataService, FileMetadataServiceImpl};
use crate::service::file_service::{FileService, FileServiceImpl};

pub mod client;
pub mod crypto;
pub mod error_enum;
pub mod model;
pub mod repo;
pub mod service;

static API_LOC: &str = "http://lockbook.app:8000";
static DB_NAME: &str = "lockbook.sled";

type DefaultCrypto = RsaCryptoService;
type DefaultDbProvider = DiskBackedDB;
type DefaultClient = ClientImpl;
type DefaultAcountRepo = AccountRepoImpl;
type DefaultAcountService = AccountServiceImpl<DefaultCrypto, DefaultAcountRepo, DefaultClient>;
type DefaultFileMetadataRepo = FileMetadataRepoImpl;
type DefaultFileRepo = FileRepoImpl;
type DefaultFileMetadataService = FileMetadataServiceImpl<
    DefaultFileMetadataRepo,
    DefaultFileRepo,
    DefaultAcountRepo,
    DefaultClient,
>;
type DefaultFileService = FileServiceImpl<DefaultFileMetadataRepo, DefaultFileRepo>;

static FAILURE_DB: &str = "FAILURE<DB_ERROR>";
static FAILURE_ACCOUNT: &str = "FAILURE<ACCOUNT_MISSING>";

static FAILURE_META_CREATE: &str = "FAILURE<META_CREATE>";
static FAILURE_META_UPDATE: &str = "FAILURE<META_UPDATE>";

static FAILURE_FILE_GET: &str = "FAILURE<FILE_GET>";
static FAILURE_FILE_CREATE: &str = "FAILURE<FILE_CREATE>";
static FAILURE_FILE_UPDATE: &str = "FAILURE<FILE_UPDATE>";

#[allow(dead_code)]
fn info(msg: String) {
    println!("ℹ️ {}", msg)
}

#[allow(dead_code)]
fn debug(msg: String) {
    println!("🚧 {}", msg)
}

#[allow(dead_code)]
fn warn(msg: String) {
    println!("⚠️ {}", msg)
}

#[allow(dead_code)]
fn error(msg: String) {
    eprintln!("🛑 {}", msg)
}

#[allow(dead_code)]
fn fatal(msg: String) {
    eprintln!("🆘 {}", msg)
}

unsafe fn string_from_ptr(c_path: *const c_char) -> String {
    CStr::from_ptr(c_path)
        .to_str()
        .expect("Could not C String -> Rust String")
        .to_string()
}

unsafe fn connect_db(c_path: *const c_char) -> Option<Db> {
    let path = string_from_ptr(c_path);
    let config = Config {
        writeable_path: path,
    };
    match DefaultDbProvider::connect_to_db(&config) {
        Ok(db) => Some(db),
        Err(err) => {
            error(format!("DB connection failed! Error: {:?}", err));
            None
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn is_db_present(c_path: *const c_char) -> c_int {
    let path = string_from_ptr(c_path);

    let db_path = path + "/" + DB_NAME;
    debug(format!("Checking if {:?} exists", db_path));
    if Path::new(db_path.as_str()).exists() {
        debug(format!("DB Exists!"));
        1
    } else {
        error(format!("DB Does not exist!"));
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn release_pointer(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    CString::from_raw(s);
}

#[no_mangle]
pub unsafe extern "C" fn get_account(c_path: *const c_char) -> *mut c_char {
    let db = match connect_db(c_path) {
        None => return CString::new(FAILURE_DB).unwrap().into_raw(),
        Some(db) => db,
    };

    match DefaultAcountRepo::get_account(&db) {
        Ok(account) => CString::new(account.username).unwrap().into_raw(),
        Err(err) => {
            error(format!("Account retrieval failed with error: {:?}", err));
            CString::new(FAILURE_ACCOUNT).unwrap().into_raw()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn create_account(c_path: *const c_char, c_username: *const c_char) -> c_int {
    let db = match connect_db(c_path) {
        None => return 0,
        Some(db) => db,
    };

    let username = string_from_ptr(c_username);

    match DefaultAcountService::create_account(&db, username.to_string()) {
        Ok(_) => 1,
        Err(err) => {
            error(format!("Account creation failed with error: {:?}", err));
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn sync_files(c_path: *const c_char, sync: bool) -> *mut c_char {
    let db = match connect_db(c_path) {
        None => return CString::new(FAILURE_DB).unwrap().into_raw(),
        Some(db) => db,
    };

    match DefaultFileMetadataService::sync(&db, sync) {
        Ok(metas) => CString::new(json!(&metas).to_string()).unwrap().into_raw(),
        Err(err) => {
            error(format!("Update metadata failed with error: {:?}", err));
            CString::new(json!([]).to_string()).unwrap().into_raw()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn create_file(
    c_path: *const c_char,
    c_file_name: *const c_char,
    c_file_path: *const c_char,
) -> *mut c_char {
    let db = match connect_db(c_path) {
        None => return CString::new(FAILURE_DB).unwrap().into_raw(),
        Some(db) => db,
    };
    let file_name = string_from_ptr(c_file_name);
    let file_path = string_from_ptr(c_file_path);

    match DefaultFileMetadataService::create(&db, file_name, file_path) {
        Ok(meta) => {
            match DefaultFileRepo::update(
                &db,
                &File {
                    id: format!("{}", meta.id),
                    content: "".to_string(),
                },
            ) {
                Ok(_) => CString::new(json!(&meta).to_string()).unwrap().into_raw(),
                Err(err) => {
                    error(format!("Failed to create file! Error: {:?}", err));
                    CString::new(FAILURE_FILE_CREATE).unwrap().into_raw()
                }
            }
        }
        Err(err) => {
            error(format!("Failed to create file metadata! Error: {:?}", err));
            CString::new(FAILURE_META_CREATE).unwrap().into_raw()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn get_file(c_path: *const c_char, c_file_id: *const c_char) -> *mut c_char {
    let db = match connect_db(c_path) {
        None => return CString::new(FAILURE_DB).unwrap().into_raw(),
        Some(db) => db,
    };
    let file_id = string_from_ptr(c_file_id);

    match DefaultFileService::get(&db, file_id) {
        Ok(file) => CString::new(json!(&file).to_string()).unwrap().into_raw(),
        Err(err) => {
            error(format!("Failed to get file! Error: {:?}", err));
            CString::new(FAILURE_FILE_GET).unwrap().into_raw()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn update_file(
    c_path: *const c_char,
    c_file_id: *const c_char,
    c_file_content: *const c_char,
) -> c_int {
    let db = match connect_db(c_path) {
        None => return 0,
        Some(db) => db,
    };
    let file_id = string_from_ptr(c_file_id);
    let file_content = string_from_ptr(c_file_content);

    match DefaultFileService::update(&db, file_id, file_content) {
        Ok(_) => 1,
        Err(err) => {
            error(format!("Failed to update file! Error: {:?}", err));
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn purge_files(c_path: *const c_char) -> c_int {
    let db = match connect_db(c_path) {
        None => return 0,
        Some(db) => db,
    };
    match DefaultFileMetadataRepo::get_all(&db) {
        Ok(metas) => metas.into_iter().for_each(|meta| {
            DefaultFileMetadataRepo::delete(&db, &meta.id).unwrap();
            ()
        }),
        Err(err) => error(format!("Failed to delete file! Error: {:?}", err)),
    }
    1
}
