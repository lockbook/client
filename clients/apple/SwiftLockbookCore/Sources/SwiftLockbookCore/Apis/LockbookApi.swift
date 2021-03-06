import Foundation

public protocol LockbookApi {
    // Account
    func getAccount() -> FfiResult<Account, GetAccountError>
    func createAccount(username: String, apiLocation: String) -> FfiResult<Empty, CreateAccountError>
    func importAccount(accountString: String) -> FfiResult<Empty, ImportError>
    func exportAccount() -> FfiResult<String, AccountExportError>
    func getUsage() -> FfiResult<UsageMetrics, GetUsageError>

    // Work
    func syncAll() -> FfiResult<Empty, SyncAllError>
    func calculateWork() -> FfiResult<WorkMetadata, CalculateWorkError>
    func setLastSynced(lastSync: UInt64) -> FfiResult<Empty, SetLastSyncedError>
    func getLastSyncedHumanString() -> FfiResult<String, GetLastSyncedError>
    func getLocalChanges() -> FfiResult<[UUID], GetLocalChangesError>
    
    // Directory
    func getRoot() -> FfiResult<ClientFileMetadata, GetRootError>
    func listFiles() -> FfiResult<[ClientFileMetadata], ListMetadatasError>
    
    // Document
    func getFile(id: UUID) -> FfiResult<String, ReadDocumentError>
    func createFile(name: String, dirId: UUID, isFolder: Bool) -> FfiResult<ClientFileMetadata, CreateFileError>
    func updateFile(id: UUID, content: String) -> FfiResult<Empty, WriteToDocumentError>
    func deleteFile(id: UUID) -> FfiResult<Empty, FileDeleteError>
    func renameFile(id: UUID, name: String) -> FfiResult<Empty, RenameFileError>
    func moveFile(id: UUID, newParent: UUID) -> FfiResult<Empty, MoveFileError>
    func readDrawing(id: UUID) -> FfiResult<Drawing, ReadDocumentError>
    func writeDrawing(id: UUID, content: Drawing) -> FfiResult<Empty, WriteToDocumentError>
    func exportDrawing(id: UUID) -> FfiResult<Data, ExportDrawingError>

    // State
    func getState() -> FfiResult<DbState, GetStateError>
    func migrateState() -> FfiResult<Empty, MigrationError>
}
