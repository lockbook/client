use crate::file_index_repo;
use crate::file_index_repo::{
    ChangeDocumentVersionAndSizeError, CreateFileError, DeleteFileError, MoveFileError,
    RenameFileError,
};
use crate::{file_content_client, RequestContext};
use lockbook_models::api::*;
use lockbook_models::file_metadata::FileType;

pub async fn change_document_content(
    context: &mut RequestContext<'_, ChangeDocumentContentRequest>,
) -> Result<ChangeDocumentContentResponse, Option<ChangeDocumentContentError>> {
    let request = &context.request;
    let server_state = &mut context.server_state;
    let mut transaction = match server_state.index_db_client.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Internal server error! Cannot begin transaction: {:?}", e);
            return Err(None);
        }
    };

    let result = file_index_repo::change_document_version_and_size(
        &mut transaction,
        request.id,
        request.new_content.value.len() as u64,
        request.old_metadata_version,
    )
    .await;

    let (old_content_version, new_version) = result.map_err(|e| match e {
        ChangeDocumentVersionAndSizeError::DoesNotExist => {
            Some(ChangeDocumentContentError::DocumentNotFound)
        }
        ChangeDocumentVersionAndSizeError::IncorrectOldVersion => {
            Some(ChangeDocumentContentError::EditConflict)
        }
        ChangeDocumentVersionAndSizeError::Deleted => {
            Some(ChangeDocumentContentError::DocumentDeleted)
        }
        ChangeDocumentVersionAndSizeError::Postgres(_)
        | ChangeDocumentVersionAndSizeError::Deserialize(_) => {
            error!(
                "Internal server error! Cannot change document content version in Postgres: {:?}",
                e
            );
            None
        }
    })?;

    let create_result = file_content_client::create(
        &server_state.files_db_client,
        request.id,
        new_version,
        &request.new_content,
    )
    .await;
    if create_result.is_err() {
        error!(
            "Internal server error! Cannot create file in S3: {:?}",
            create_result
        );
        return Err(None);
    };

    let delete_result = file_content_client::delete(
        &server_state.files_db_client,
        request.id,
        old_content_version,
    )
    .await;
    if delete_result.is_err() {
        error!(
            "Internal server error! Cannot delete file in S3: {:?}",
            delete_result
        );
        return Err(None);
    };

    match transaction.commit().await {
        Ok(()) => Ok(ChangeDocumentContentResponse {
            new_metadata_and_content_version: new_version,
        }),
        Err(e) => {
            error!("Internal server error! Cannot commit transaction: {:?}", e);
            Err(None)
        }
    }
}

pub async fn create_document(
    context: &mut RequestContext<'_, CreateDocumentRequest>,
) -> Result<CreateDocumentResponse, Option<CreateDocumentError>> {
    let request = &context.request;
    let server_state = &mut context.server_state;
    let mut transaction = match server_state.index_db_client.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Internal server error! Cannot begin transaction: {:?}", e);
            return Err(None);
        }
    };

    let index_result = file_index_repo::create_file(
        &mut transaction,
        request.id,
        request.parent,
        FileType::Document,
        &request.name,
        &context.public_key,
        &request.parent_access_key,
        Some(request.content.value.len() as u64),
    )
    .await;
    let new_version = index_result.map_err(|e| match e {
        CreateFileError::IdTaken => Some(CreateDocumentError::FileIdTaken),
        CreateFileError::PathTaken => Some(CreateDocumentError::DocumentPathTaken),
        CreateFileError::OwnerDoesNotExist => Some(CreateDocumentError::UserNotFound),
        CreateFileError::ParentDoesNotExist => Some(CreateDocumentError::ParentNotFound),
        CreateFileError::Postgres(_) | CreateFileError::Serialize(_) => {
            error!(
                "Internal server error! Cannot create document in Postgres: {:?}",
                e
            );
            None
        }
    })?;

    let files_result = file_content_client::create(
        &server_state.files_db_client,
        request.id,
        new_version,
        &request.content,
    )
    .await;

    if files_result.is_err() {
        error!(
            "Internal server error! Cannot create file in S3: {:?}",
            files_result
        );
        return Err(None);
    };

    match transaction.commit().await {
        Ok(()) => Ok(CreateDocumentResponse {
            new_metadata_and_content_version: new_version,
        }),
        Err(e) => {
            error!("Internal server error! Cannot commit transaction: {:?}", e);
            Err(None)
        }
    }
}

pub async fn delete_document(
    context: &mut RequestContext<'_, DeleteDocumentRequest>,
) -> Result<DeleteDocumentResponse, Option<DeleteDocumentError>> {
    let request = &context.request;
    let server_state = &mut context.server_state;
    let mut transaction = match server_state.index_db_client.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Internal server error! Cannot begin transaction: {:?}", e);
            return Err(None);
        }
    };

    let index_result = file_index_repo::delete_file(&mut transaction, request.id).await;
    let index_responses = index_result.map_err(|e| match e {
        DeleteFileError::DoesNotExist => Some(DeleteDocumentError::DocumentNotFound),
        DeleteFileError::Deleted => Some(DeleteDocumentError::DocumentDeleted),
        DeleteFileError::IllegalRootChange
        | DeleteFileError::Postgres(_)
        | DeleteFileError::Serialize(_)
        | DeleteFileError::Deserialize(_)
        | DeleteFileError::UuidDeserialize(_) => {
            error!(
                "Internal server error! Cannot delete document in Postgres: {:?}",
                e
            );
            None
        }
    })?;

    let single_index_response = if let Some(result) = index_responses.iter().last() {
        result
    } else {
        error!("Internal server error! Unexpected zero or multiple postgres rows");
        return Err(None);
    };

    let files_result = file_content_client::delete(
        &server_state.files_db_client,
        request.id,
        single_index_response.old_content_version,
    )
    .await;
    if files_result.is_err() {
        error!(
            "Internal server error! Cannot delete file in S3: {:?}",
            files_result
        );
        return Err(None);
    };

    match transaction.commit().await {
        Ok(()) => Ok(DeleteDocumentResponse {
            new_metadata_and_content_version: single_index_response.new_metadata_version,
        }),
        Err(e) => {
            error!("Internal server error! Cannot commit transaction: {:?}", e);
            Err(None)
        }
    }
}

pub async fn move_document(
    context: &mut RequestContext<'_, MoveDocumentRequest>,
) -> Result<MoveDocumentResponse, Option<MoveDocumentError>> {
    let request = &context.request;
    let server_state = &mut context.server_state;
    let mut transaction = match server_state.index_db_client.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Internal server error! Cannot begin transaction: {:?}", e);
            return Err(None);
        }
    };

    let result = file_index_repo::move_file(
        &mut transaction,
        request.id,
        request.old_metadata_version,
        request.new_parent,
        request.new_folder_access.clone(),
    )
    .await;
    let new_version = result.map_err(|e| match e {
        MoveFileError::DoesNotExist => Some(MoveDocumentError::DocumentNotFound),
        MoveFileError::IncorrectOldVersion => Some(MoveDocumentError::EditConflict),
        MoveFileError::Deleted => Some(MoveDocumentError::DocumentDeleted),
        MoveFileError::PathTaken => Some(MoveDocumentError::DocumentPathTaken),
        MoveFileError::ParentDoesNotExist => Some(MoveDocumentError::ParentNotFound),
        MoveFileError::ParentDeleted => Some(MoveDocumentError::ParentDeleted),
        MoveFileError::FolderMovedIntoDescendants
        | MoveFileError::IllegalRootChange
        | MoveFileError::Postgres(_)
        | MoveFileError::Serialize(_) => {
            error!(
                "Internal server error! Cannot move document in Postgres: {:?}",
                e
            );
            None
        }
    })?;

    match transaction.commit().await {
        Ok(()) => Ok(MoveDocumentResponse {
            new_metadata_version: new_version,
        }),
        Err(e) => {
            error!("Internal server error! Cannot commit transaction: {:?}", e);
            Err(None)
        }
    }
}

pub async fn rename_document(
    context: &mut RequestContext<'_, RenameDocumentRequest>,
) -> Result<RenameDocumentResponse, Option<RenameDocumentError>> {
    let request = &context.request;
    let server_state = &mut context.server_state;
    let mut transaction = match server_state.index_db_client.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Internal server error! Cannot begin transaction: {:?}", e);
            return Err(None);
        }
    };

    let result = file_index_repo::rename_file(
        &mut transaction,
        request.id,
        request.old_metadata_version,
        FileType::Document,
        &request.new_name,
    )
    .await;
    let new_version = result.map_err(|e| match e {
        RenameFileError::DoesNotExist => Some(RenameDocumentError::DocumentNotFound),
        RenameFileError::IncorrectOldVersion => Some(RenameDocumentError::EditConflict),
        RenameFileError::Deleted => Some(RenameDocumentError::DocumentDeleted),
        RenameFileError::PathTaken => Some(RenameDocumentError::DocumentPathTaken),
        RenameFileError::IllegalRootChange
        | RenameFileError::Postgres(_)
        | RenameFileError::Serialize(_) => {
            error!(
                "Internal server error! Cannot rename document in Postgres: {:?}",
                e
            );
            None
        }
    })?;

    match transaction.commit().await {
        Ok(()) => Ok(RenameDocumentResponse {
            new_metadata_version: new_version,
        }),
        Err(e) => {
            error!("Internal server error! Cannot commit transaction: {:?}", e);
            Err(None)
        }
    }
}

pub async fn get_document(
    context: &mut RequestContext<'_, GetDocumentRequest>,
) -> Result<GetDocumentResponse, Option<GetDocumentError>> {
    let request = &context.request;
    let server_state = &mut context.server_state;
    let files_result = file_content_client::get(
        &server_state.files_db_client,
        request.id,
        request.content_version,
    )
    .await;
    match files_result {
        Ok(c) => Ok(GetDocumentResponse { content: c }),
        Err(file_content_client::Error::NoSuchKey(_)) => {
            Err(Some(GetDocumentError::DocumentNotFound))
        }
        Err(e) => {
            error!("Internal server error! Cannot get file from S3: {:?}", e);
            Err(None)
        }
    }
}

pub async fn create_folder(
    context: &mut RequestContext<'_, CreateFolderRequest>,
) -> Result<CreateFolderResponse, Option<CreateFolderError>> {
    let request = &context.request;
    let server_state = &mut context.server_state;
    let mut transaction = match server_state.index_db_client.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Internal server error! Cannot begin transaction: {:?}", e);
            return Err(None);
        }
    };

    let result = file_index_repo::create_file(
        &mut transaction,
        request.id,
        request.parent,
        FileType::Folder,
        &request.name,
        &context.public_key,
        &request.parent_access_key,
        None,
    )
    .await;
    let new_version = result.map_err(|e| match e {
        CreateFileError::IdTaken => Some(CreateFolderError::FileIdTaken),
        CreateFileError::PathTaken => Some(CreateFolderError::FolderPathTaken),
        CreateFileError::OwnerDoesNotExist => Some(CreateFolderError::UserNotFound),
        CreateFileError::ParentDoesNotExist => Some(CreateFolderError::ParentNotFound),
        CreateFileError::Postgres(_) | CreateFileError::Serialize(_) => {
            error!(
                "Internal server error! Cannot create folder in Postgres: {:?}",
                e
            );
            None
        }
    })?;

    match transaction.commit().await {
        Ok(()) => Ok(CreateFolderResponse {
            new_metadata_version: new_version,
        }),
        Err(e) => {
            error!("Internal server error! Cannot commit transaction: {:?}", e);
            Err(None)
        }
    }
}

pub async fn delete_folder(
    context: &mut RequestContext<'_, DeleteFolderRequest>,
) -> Result<DeleteFolderResponse, Option<DeleteFolderError>> {
    let request = &context.request;
    let server_state = &mut context.server_state;
    let mut transaction = match server_state.index_db_client.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Internal server error! Cannot begin transaction: {:?}", e);
            return Err(None);
        }
    };

    let index_result = file_index_repo::delete_file(&mut transaction, request.id).await;
    let index_responses = index_result.map_err(|e| match e {
        DeleteFileError::DoesNotExist => Some(DeleteFolderError::FolderNotFound),
        DeleteFileError::Deleted => Some(DeleteFolderError::FolderDeleted),
        DeleteFileError::IllegalRootChange => Some(DeleteFolderError::CannotDeleteRoot),
        DeleteFileError::Postgres(_)
        | DeleteFileError::Serialize(_)
        | DeleteFileError::Deserialize(_)
        | DeleteFileError::UuidDeserialize(_) => {
            error!(
                "Internal server error! Cannot delete folder in Postgres: {:?}",
                e
            );
            None
        }
    })?;

    let root_result = if let Some(result) =
        index_responses.iter().filter(|r| r.id == request.id).last()
    {
        result
    } else {
        error!("Internal server error! Unexpected zero or multiple postgres rows for delete folder root");
        return Err(None);
    };

    for r in index_responses.iter() {
        if !r.is_folder {
            let files_result = file_content_client::delete(
                &server_state.files_db_client,
                r.id,
                r.old_content_version,
            )
            .await;
            if files_result.is_err() {
                error!(
                    "Internal server error! Cannot delete file in S3: {:?}",
                    files_result
                );
                return Err(None);
            };
        }
    }

    match transaction.commit().await {
        Ok(()) => Ok(DeleteFolderResponse {
            new_metadata_version: root_result.new_metadata_version,
        }),
        Err(e) => {
            error!("Internal server error! Cannot commit transaction: {:?}", e);
            Err(None)
        }
    }
}

pub async fn move_folder(
    context: &mut RequestContext<'_, MoveFolderRequest>,
) -> Result<MoveFolderResponse, Option<MoveFolderError>> {
    let request = &context.request;
    let server_state = &mut context.server_state;
    let mut transaction = match server_state.index_db_client.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Internal server error! Cannot begin transaction: {:?}", e);
            return Err(None);
        }
    };

    let result = file_index_repo::move_file(
        &mut transaction,
        request.id,
        request.old_metadata_version,
        request.new_parent,
        request.new_folder_access.clone(),
    )
    .await;
    let new_version = result.map_err(|e| match e {
        MoveFileError::DoesNotExist => Some(MoveFolderError::FolderNotFound),
        MoveFileError::IncorrectOldVersion => Some(MoveFolderError::EditConflict),
        MoveFileError::Deleted => Some(MoveFolderError::FolderDeleted),
        MoveFileError::PathTaken => Some(MoveFolderError::FolderPathTaken),
        MoveFileError::ParentDoesNotExist => Some(MoveFolderError::ParentNotFound),
        MoveFileError::ParentDeleted => Some(MoveFolderError::ParentDeleted),
        MoveFileError::FolderMovedIntoDescendants => {
            Some(MoveFolderError::CannotMoveIntoDescendant)
        }
        MoveFileError::IllegalRootChange => Some(MoveFolderError::CannotMoveRoot),
        MoveFileError::Postgres(_) | MoveFileError::Serialize(_) => {
            error!(
                "Internal server error! Cannot move folder in Postgres: {:?}",
                e
            );
            None
        }
    })?;

    match transaction.commit().await {
        Ok(()) => Ok(MoveFolderResponse {
            new_metadata_version: new_version,
        }),
        Err(e) => {
            error!("Internal server error! Cannot commit transaction: {:?}", e);
            Err(None)
        }
    }
}

pub async fn rename_folder(
    context: &mut RequestContext<'_, RenameFolderRequest>,
) -> Result<RenameFolderResponse, Option<RenameFolderError>> {
    let request = &context.request;
    let server_state = &mut context.server_state;
    let mut transaction = match server_state.index_db_client.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Internal server error! Cannot begin transaction: {:?}", e);
            return Err(None);
        }
    };

    let result = file_index_repo::rename_file(
        &mut transaction,
        request.id,
        request.old_metadata_version,
        FileType::Folder,
        &request.new_name,
    )
    .await;
    let new_version = result.map_err(|e| match e {
        RenameFileError::DoesNotExist => Some(RenameFolderError::FolderNotFound),
        RenameFileError::IncorrectOldVersion => Some(RenameFolderError::EditConflict),
        RenameFileError::Deleted => Some(RenameFolderError::FolderDeleted),
        RenameFileError::PathTaken => Some(RenameFolderError::FolderPathTaken),
        RenameFileError::IllegalRootChange => Some(RenameFolderError::CannotRenameRoot),
        RenameFileError::Postgres(_) | RenameFileError::Serialize(_) => {
            error!(
                "Internal server error! Cannot rename folder in Postgres: {:?}",
                e
            );
            None
        }
    })?;

    match transaction.commit().await {
        Ok(()) => Ok(RenameFolderResponse {
            new_metadata_version: new_version,
        }),
        Err(e) => {
            error!("Internal server error! Cannot commit transaction: {:?}", e);
            Err(None)
        }
    }
}

pub async fn get_updates(
    context: &mut RequestContext<'_, GetUpdatesRequest>,
) -> Result<GetUpdatesResponse, Option<GetUpdatesError>> {
    let request = &context.request;
    let server_state = &mut context.server_state;
    let mut transaction = match server_state.index_db_client.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Internal server error! Cannot begin transaction: {:?}", e);
            return Err(None);
        }
    };
    let result = file_index_repo::get_updates(
        &mut transaction,
        &context.public_key,
        request.since_metadata_version,
    )
    .await;
    let updates = result.map_err(|e| {
        error!(
            "Internal server error! Cannot get updates from Postgres: {:?}",
            e
        );
        None
    })?;

    match transaction.commit().await {
        Ok(()) => Ok(GetUpdatesResponse {
            file_metadata: updates,
        }),
        Err(e) => {
            error!("Internal server error! Cannot commit transaction: {:?}", e);
            Err(None)
        }
    }
}