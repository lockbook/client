use crate::model::client_conversion::{generate_client_work_unit, ClientWorkUnit};
use crate::model::repo::RepoSource;
use crate::model::state::Config;
use crate::repo::{account_repo, document_repo, file_repo, last_updated_repo, metadata_repo};
use crate::service::file_compression_service;
use crate::service::{file_encryption_service, file_service};
use crate::{client, CoreError};
use lockbook_models::account::Account;
use lockbook_models::api::{
    ChangeDocumentContentRequest, FileMetadataUpsertsRequest, GetDocumentRequest, GetUpdatesRequest,
};
use lockbook_models::file_metadata::FileMetadata;
use lockbook_models::file_metadata::FileType::{Document, Folder};
use lockbook_models::work_unit::WorkUnit;
use lockbook_models::work_unit::WorkUnit::{LocalChange, ServerChange};
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Clone)]
pub struct WorkCalculated {
    pub work_units: Vec<WorkUnit>,
    pub most_recent_update_from_server: u64,
}

pub struct SyncProgress {
    pub total: usize,
    pub progress: usize,
    pub current_work_unit: ClientWorkUnit,
}

pub fn calculate_work(config: &Config) -> Result<WorkCalculated, CoreError> {
    // todo: new definition of work
    info!("Calculating Work");
    let mut work_units: Vec<WorkUnit> = vec![];

    let account = account_repo::get(config)?;
    let last_sync = last_updated_repo::get(config)?;

    let server_updates = client::request(
        &account,
        GetUpdatesRequest {
            since_metadata_version: last_sync,
        },
    )
    .map_err(CoreError::from)?
    .file_metadata;

    let mut most_recent_update_from_server: u64 = last_sync;
    for metadata in server_updates {
        if metadata.metadata_version > most_recent_update_from_server {
            most_recent_update_from_server = metadata.metadata_version;
        }

        match file_repo::maybe_get_metadata(config, metadata.id)? {
            None => {
                if !metadata.deleted {
                    // no work for files we don't have that have been deleted
                    work_units.push(ServerChange { metadata })
                }
            }
            Some((local_metadata, _)) => {
                if metadata.metadata_version != local_metadata.metadata_version {
                    work_units.push(ServerChange { metadata })
                }
            }
        };
    }

    work_units.sort_by(|f1, f2| {
        f1.get_metadata()
            .metadata_version
            .cmp(&f2.get_metadata().metadata_version)
    });

    let changes = file_repo::get_all_metadata(config)?.union_new_and_modified();
    for change_description in changes {
        let (metadata, _) = file_repo::get_metadata(config, change_description.id)?;
        work_units.push(LocalChange { metadata });
    }
    debug!("Work Calculated: {:#?}", work_units);

    Ok(WorkCalculated {
        work_units,
        most_recent_update_from_server,
    })
}

pub fn execute_work(config: &Config, account: &Account, work: WorkUnit) -> Result<(), CoreError> {
    match work {
        WorkUnit::LocalChange { mut metadata } => {
            handle_local_change(config, &account, &mut metadata)
        }
        WorkUnit::ServerChange { mut metadata } => {
            handle_server_change(config, &account, &mut metadata)
        }
    }
}

fn merge_maybe_metadata(
    base: Option<FileMetadata>,
    local: Option<FileMetadata>,
    remote: Option<FileMetadata>,
) -> Option<FileMetadata> {
}

fn merge_metadata(base: FileMetadata, local: FileMetadata, remote: FileMetadata) -> FileMetadata {
    let local_renamed = local.name.hmac != base.name.hmac;
    let remote_renamed = remote.name.hmac != base.name.hmac;
    let name = match (local_renamed, remote_renamed) {
        (false, false) => base.name,
        (true, false) => local.name,
        (false, true) => remote.name,
        (true, true) => {
            remote.name // resolve rename conflicts in favor of remote
        }
    };

    let local_moved = local.parent != base.parent;
    let remote_moved = remote.parent != remote.parent;
    let parent = match (local_moved, remote_moved) {
        (false, false) => base.parent,
        (true, false) => local.parent,
        (false, true) => remote.parent,
        (true, true) => {
            remote.parent // resolve move conflicts in favor of remote
        }
    };

    let deleted = base.deleted || local.deleted || remote.deleted; // resolve delete conflicts by deleting

    FileMetadata {
        id: base.id,
        file_type: base.file_type,
        parent,
        name,
        owner: base.owner,
        metadata_version: remote.metadata_version,
        content_version: remote.content_version,
        deleted,
        user_access_keys: base.user_access_keys,
        folder_access_keys: base.folder_access_keys,
    }
}

// fn merge_documents(base: )

fn pull(
    config: &Config,
    account: &Account,
    f: Option<Box<dyn Fn(SyncProgress)>>,
) -> Result<(), CoreError> {
    // pull remote changes
    let last_sync = last_updated_repo::get(config)?;
    let remote = client::request(
        &account,
        GetUpdatesRequest {
            since_metadata_version: last_sync,
        },
    )
    .map_err(CoreError::from)?
    .file_metadata;

    for remote in remote {
        if remote.deleted {
            file_repo::delete(config, RepoSource::Remote, remote.id)?;
        }

        let maybe_base = metadata_repo::maybe_get(config, RepoSource::Remote, remote.id)?;
        let maybe_local = metadata_repo::maybe_get(config, RepoSource::Local, remote.id)?;
        match (maybe_base, maybe_local) {
            (None, None) => {
                // new file
            }
            (Some(base), None) => {
                // update to unmodified file
            }
            (None, Some(local)) => {
                // new local file with the same id
                return Err(CoreError::Unexpected(String::from(
                    "new file from server with same id as new local file",
                )));
            }
            (Some(base), Some(local)) => {
                // update to modified file
                let merged = merge_metadata(base, local, remote);
            }
        }
    }

    // merge with local changes; save results locally
    let local_changes = file_repo::get_all_metadata(config)?.modified;
    let conflicts = local_changes
        .iter()
        .filter(|l| remote.iter().find(|r| l.id == r.id).is_some());
    for conflict in conflicts {
        // todo
        // * what's going on with versions?
        // * do we have to process deletes after moves?
    }

    Ok(())
}

fn push(
    config: &Config,
    account: &Account,
    f: Option<Box<dyn Fn(SyncProgress)>>,
) -> Result<(), CoreError> {
    // push local metadata changes
    client::request(
        &account,
        FileMetadataUpsertsRequest {
            updates: file_repo::get_all_metadata_changes(config)?,
        },
    )
    .map_err(CoreError::from)?;

    // push local content changes
    for id in file_repo::get_all_with_document_changes(config)? {
        client::request(
            &account,
            ChangeDocumentContentRequest {
                id: id,
                old_metadata_version: file_repo::get_metadata(config, id)?.0.metadata_version,
                new_content: file_repo::get_document(config, id)?.0,
            },
        )
        .map_err(CoreError::from)?;
    }

    Ok(())
}

pub fn sync(config: &Config, f: Option<Box<dyn Fn(SyncProgress)>>) -> Result<(), CoreError> {
    let account = account_repo::get(config)?;

    let account = account_repo::get(config)?;
    let mut sync_errors: HashMap<Uuid, CoreError> = HashMap::new();
    let mut work_calculated = calculate_work(config)?;

    // Retry sync n times
    for _ in 0..10 {
        info!("Syncing");

        for (progress, work_unit) in work_calculated.work_units.iter().enumerate() {
            if let Some(ref func) = f {
                func(SyncProgress {
                    total: work_calculated.work_units.len(),
                    progress,
                    current_work_unit: generate_client_work_unit(config, &work_unit)?,
                })
            }

            match execute_work(config, &account, work_unit.clone()) {
                Ok(_) => {
                    debug!("{:#?} executed successfully", work_unit);
                    sync_errors.remove(&work_unit.get_metadata().id);
                }
                Err(err) => {
                    error!("Sync error detected: {:#?} {:#?}", work_unit, err);
                    sync_errors.insert(work_unit.get_metadata().id, err);
                }
            }
        }

        if sync_errors.is_empty() {
            last_updated_repo::set(config, work_calculated.most_recent_update_from_server)?;
        }

        work_calculated = calculate_work(config)?;
        if work_calculated.work_units.is_empty() {
            break;
        }
    }

    if sync_errors.is_empty() {
        last_updated_repo::set(config, work_calculated.most_recent_update_from_server)?;
        Ok(())
    } else {
        error!("We finished everything calculate work told us about, but still have errors, this is concerning, the errors are: {:#?}", sync_errors);
        Err(CoreError::Unexpected(format!(
            "work execution errors: {:#?}",
            sync_errors
        )))
    }
}

/// Paths within lockbook must be unique. Prior to handling a server change we make sure that
/// there are not going to be path conflicts. If there are, we find the file that is conflicting
/// locally and rename it
fn rename_local_conflicting_files(
    config: &Config,
    metadata: &FileMetadata,
) -> Result<(), CoreError> {
    let conflicting_files = file_repo::get_children(config, metadata.parent)?
        .into_iter()
        .filter(|potential_conflict| potential_conflict.name == metadata.name)
        .filter(|potential_conflict| potential_conflict.id != metadata.id)
        .collect::<Vec<FileMetadata>>();

    // There should only be one of these
    for conflicting_file in conflicting_files {
        let old_name = file_encryption_service::get_name(&config, &conflicting_file)?;
        file_service::rename(
            config,
            conflicting_file.id,
            &format!("{}-NAME-CONFLICT-{}", old_name, conflicting_file.id),
        )?
    }

    Ok(())
}

/// Save metadata locally, and download the file contents if this file is a Document
fn save_file_locally(
    config: &Config,
    account: &Account,
    metadata: &FileMetadata,
) -> Result<(), CoreError> {
    file_repo::insert_metadata(config, RepoSource::Remote, &metadata)?;

    if metadata.file_type == Document {
        let document = client::request(
            &account,
            GetDocumentRequest {
                id: metadata.id,
                content_version: metadata.content_version,
            },
        )
        .map_err(CoreError::from)?
        .content;

        document_repo::insert(config, RepoSource::Remote, metadata.id, &document)?;
    }

    Ok(())
}

fn delete_file_locally(config: &Config, metadata: &FileMetadata) -> Result<(), CoreError> {
    file_repo::delete(config, RepoSource::Remote, metadata.id)
}

fn merge_documents(
    config: &Config,
    account: &Account,
    metadata: &mut FileMetadata,
    local_metadata: &FileMetadata,
    local_changes: &LocalChangeRepoLocalChange,
    edited_locally: &Edited,
) -> Result<(), CoreError> {
    let local_name = file_encryption_service::get_name(&config, &local_metadata)?;
    if local_name.ends_with(".md") || local_name.ends_with(".txt") {
        let common_ancestor = {
            let compressed_common_ancestor = file_encryption_service::user_read_document(
                &account,
                &edited_locally.old_value,
                &edited_locally.access_info,
            )?;

            file_compression_service::decompress(&compressed_common_ancestor)?
        };

        let current_version = file_service::read_document(config, metadata.id)?;

        let server_version = {
            let server_document = client::request(
                &account,
                GetDocumentRequest {
                    id: metadata.id,
                    content_version: metadata.content_version,
                },
            )?
            .content;

            let compressed_server_version = file_encryption_service::user_read_document(
                &account,
                &server_document,
                &edited_locally.access_info,
            )?;
            // This assumes that a file is never re-keyed.

            file_compression_service::decompress(&compressed_server_version)?
        };

        let result = match diffy::merge_bytes(&common_ancestor, &current_version, &server_version) {
            Ok(no_conflicts) => no_conflicts,
            Err(conflicts) => conflicts,
        };

        file_service::write_document(config, metadata.id, &result)?;
    } else {
        // Create a new file
        let new_file = file_service::create(
            config,
            &format!("{}-CONTENT-CONFLICT-{}", &local_name, local_metadata.id),
            local_metadata.parent,
            Document,
        )?;

        // Copy the local copy over
        remote_document_repo::insert(
            config,
            new_file.id,
            &remote_document_repo::get(config, local_changes.id)?,
        )?;

        // Overwrite local file with server copy
        let new_content = client::request(
            &account,
            GetDocumentRequest {
                id: metadata.id,
                content_version: metadata.content_version,
            },
        )
        .map_err(CoreError::from)?
        .content;

        remote_document_repo::insert(config, metadata.id, &new_content)?;

        // Mark content as synced
        metadata_repo::untrack_edit(config, metadata.id)?;
    }

    Ok(())
}

fn merge_files(
    config: &Config,
    account: &Account,
    metadata: &mut FileMetadata,
    local_metadata: &FileMetadata,
    local_changes: &LocalChangeRepoLocalChange,
) -> Result<(), CoreError> {
    if let Some(renamed_locally) = &local_changes.renamed {
        // Check if both renamed, if so, server wins
        let server_name = file_encryption_service::get_name(&config, &metadata)?;
        if server_name != renamed_locally.old_value {
            metadata_repo::untrack_rename(config, metadata.id)?;
        } else {
            metadata.name = local_metadata.name.clone();
        }
    }

    // We moved it locally
    if let Some(moved_locally) = &local_changes.moved {
        // Check if both moved, if so server wins
        if metadata.parent != moved_locally.old_value {
            metadata_repo::untrack_move(config, metadata.id)?;
        } else {
            metadata.parent = local_metadata.parent;
            metadata.folder_access_keys = local_metadata.folder_access_keys.clone();
        }
    }

    if let Some(edited_locally) = &local_changes.content_edited {
        info!("Content conflict for: {}", metadata.id);
        if local_metadata.content_version != metadata.content_version {
            if metadata.file_type == Folder {
                // Should be unreachable
                error!("Not only was a folder edited, it was edited according to the server as well. This should not be possible, id: {}", metadata.id);
            }

            merge_documents(
                &config,
                &account,
                metadata,
                &local_metadata,
                &local_changes,
                edited_locally,
            )?;
        }
    }

    Ok(())
}

fn handle_server_change(
    config: &Config,
    account: &Account,
    metadata: &mut FileMetadata,
) -> Result<(), CoreError> {
    rename_local_conflicting_files(&config, &metadata)?;

    match remote_metadata_repo::maybe_get(config, metadata.id)? {
        None => {
            if !metadata.deleted {
                save_file_locally(&config, &account, &metadata)?;
            } else {
                debug!(
                    "Server deleted a file we don't know about, ignored. id: {:?}",
                    metadata.id
                );
            }
        }
        Some(local_metadata) => {
            match metadata_repo::get_local_changes(config, metadata.id)? {
                None => {
                    if metadata.deleted {
                        delete_file_locally(&config, &metadata)?;
                    } else {
                        save_file_locally(&config, &account, &metadata)?;
                    }
                }
                Some(local_changes) => {
                    if !local_changes.deleted && !metadata.deleted {
                        merge_files(&config, &account, metadata, &local_metadata, &local_changes)?;

                        remote_metadata_repo::insert(config, &metadata)?;
                    } else if metadata.deleted {
                        // Adding checks here is how you can protect local state from being deleted
                        delete_file_locally(&config, &metadata)?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_local_change(
    config: &Config,
    account: &Account,
    metadata: &mut FileMetadata,
) -> Result<(), CoreError> {
    match metadata_repo::get_local_changes(config, metadata.id)? {
                None => debug!("Calculate work indicated there was work to be done, but local_changes_repo didn't give us anything. It must have been unset by a server change. id: {:?}", metadata.id),
                Some(mut local_change) => {
                    if local_change.new {
                        if metadata.file_type == Document {
                            let content = remote_document_repo::get(config, metadata.id)?;
                            let version = client::request(
                                &account,
                                CreateDocumentRequest::new(&metadata, content),
                            )
                                .map_err(CoreError::from)?
                                .new_metadata_and_content_version;

                            metadata.metadata_version = version;
                            metadata.content_version = version;
                        } else {
                            let version = client::request(
                                &account,
                                CreateFolderRequest::new(&metadata),
                            )
                                .map_err(CoreError::from)?
                                .new_metadata_version;

                            metadata.metadata_version = version;
                            metadata.content_version = version;
                        }

                        remote_metadata_repo::insert(config, &metadata)?;

                        metadata_repo::untrack_new_file(config, metadata.id)?;
                        local_change.new = false;
                        local_change.renamed = None;
                        local_change.content_edited = None;
                        local_change.moved = None;

                        // return early to allow any other child operations like move can be sent to the
                        // server
                        if local_change.deleted && metadata.file_type == Folder {
                            return Ok(());
                        }
                    }

                    if local_change.renamed.is_some() {
                        let version = if metadata.file_type == Document {
                            client::request(&account, RenameDocumentRequest::new(&metadata))
                                .map_err(CoreError::from)?.new_metadata_version
                        } else {
                            client::request(&account, RenameFolderRequest::new(&metadata))
                                .map_err(CoreError::from)?.new_metadata_version
                        };
                        metadata.metadata_version = version;
                        remote_metadata_repo::insert(config, &metadata)?;

                        metadata_repo::untrack_rename(config, metadata.id)?;
                        local_change.renamed = None;
                    }

                    if local_change.moved.is_some() {
                        metadata.metadata_version = if metadata.file_type == Document {
                            client::request(&account, RenameDocumentRequest::new(&metadata))
                                .map_err(CoreError::from)?.new_metadata_version
                        } else {
                            client::request(&account, RenameFolderRequest::new(&metadata))
                                .map_err(CoreError::from)?.new_metadata_version
                        };

                        let version = if metadata.file_type == Document {
                            client::request(&account, MoveDocumentRequest::new(&metadata)).map_err(CoreError::from)?.new_metadata_version
                        } else {
                            client::request(&account, MoveFolderRequest::new(&metadata)).map_err(CoreError::from)?.new_metadata_version
                        };

                        metadata.metadata_version = version;
                        remote_metadata_repo::insert(config, &metadata)?;

                        metadata_repo::untrack_move(config, metadata.id)?;
                        local_change.moved = None;
                    }

                    if local_change.content_edited.is_some() && metadata.file_type == Document {
                        let version = client::request(&account, ChangeDocumentContentRequest{
                            id: metadata.id,
                            old_metadata_version: metadata.metadata_version,
                            new_content: remote_document_repo::get(config, metadata.id)?,
                        }).map_err(CoreError::from)?.new_metadata_and_content_version;

                        metadata.content_version = version;
                        metadata.metadata_version = version;
                        remote_metadata_repo::insert(config, &metadata)?;

                        metadata_repo::untrack_edit(config, metadata.id)?;
                        local_change.content_edited = None;
                    }

                    if local_change.deleted {
                        if metadata.file_type == Document {
                            client::request(&account, DeleteDocumentRequest{ id: metadata.id }).map_err(CoreError::from)?;
                        } else {
                            client::request(&account, DeleteFolderRequest{ id: metadata.id }).map_err(CoreError::from)?;
                        }

                        metadata_repo::delete(config, metadata.id)?;
                        local_change.deleted = false;

                        remote_metadata_repo::delete_non_recursive(config, metadata.id)?; // Now it's safe to delete this locally
                    }
                }
            }
    Ok(())
}
