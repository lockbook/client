use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::{fs, io};

use uuid::Uuid;

use crate::utils::{connect_to_db, edit_file_with_editor, get_account, get_editor};

use lockbook_core::repo::file_metadata_repo::FileMetadataRepo;

use lockbook_core::service::file_service::{FileService, NewFileFromPathError};

use lockbook_core::model::crypto::DecryptedValue;
use lockbook_core::service::sync_service::SyncService;
use lockbook_core::{DefaultFileMetadataRepo, DefaultFileService, DefaultSyncService};

pub fn new() {
    let db = connect_to_db();
    get_account(&db);

    let file_location = format!("/tmp/{}", Uuid::new_v4().to_string());
    let temp_file_path = Path::new(file_location.as_str());
    File::create(&temp_file_path)
        .expect(format!("Could not create temporary file: {}", &file_location).as_str());

    print!("Enter a filepath: ");
    io::stdout().flush().unwrap();

    let mut file_name = String::new();
    io::stdin()
        .read_line(&mut file_name)
        .expect("Failed to read from stdin");
    file_name.retain(|c| !c.is_whitespace());
    println!("Creating file {}", &file_name);

    let file_metadata = match DefaultFileService::create_at_path(&db, &file_name) {
        Ok(file_metadata) => file_metadata,
        Err(error) => match error {
            NewFileFromPathError::InvalidRootFolder => panic!("The first component of your file path does not match the name of your root folder!"),
            NewFileFromPathError::DbError(_) |
            NewFileFromPathError::NoRoot |
            NewFileFromPathError::FailedToCreateChild(_) => panic!("Unexpected error ocurred: {:?}", error)
        },
    };

    let edit_was_successful = edit_file_with_editor(&file_location);

    if edit_was_successful {
        let secret =
            fs::read_to_string(temp_file_path).expect("Could not read file that was edited");

        DefaultFileService::write_document(&db, file_metadata.id, &DecryptedValue { secret })
            .expect("Unexpected error while updating internal state");

        println!("Updating local state.");
        DefaultFileMetadataRepo::insert(&db, &file_metadata).expect("Failed to index new file!");

        println!("Syncing");
        DefaultSyncService::sync(&db).expect("Failed to sync");

        println!("Sync successful, cleaning up.")
    } else {
        eprintln!("Your editor indicated a problem, aborting and cleaning up");
    }

    fs::remove_file(&temp_file_path)
        .expect(format!("Failed to delete temporary file: {}", &file_location).as_str());
}
