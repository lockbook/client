import Foundation
import SwiftLockbookCore
import SwiftUI
import Combine

class GlobalState: ObservableObject {
    let documenstDirectory: String
    let api: LockbookApi
    @Published var state: DbState               // Handles post update logic
    @Published var account: Account?            // Determines whether to show onboarding or the main view
    @Published var globalError: AnyFfiError?    // Shows modals for unhandled errors
    @Published var files: [FileMetadata] = []   // What the file tree displays
    @Published var root: FileMetadata?          // What the file tree displays
    @Published var syncing: Bool = false {      // Setting this to true kicks off a sync
        didSet {
            if oldValue == false && syncing == true {
                serialQueue.async {
                    self.syncChannel.send(self.api.syncAll())
                }
            }
        }
    }
    let timer = Timer.publish(every: 30, on: .main, in: .common).autoconnect()
    let serialQueue = DispatchQueue(label: "syncQueue")
    
    private var syncChannel = PassthroughSubject<FfiResult<SwiftLockbookCore.Empty, SyncAllError>, Never>()
    private var cancellableSet: Set<AnyCancellable> = []
    
    func load() {
        switch api.getAccount() {
        case .success(let acc):
            account = acc
        case .failure(let err):
            handleError(err)
        }
    }
    
    func migrate() {
        let res = api.migrateState()
            .eraseError()
            .flatMap(transform: { _ in api.getState().eraseError() })
        switch res {
        case .success(let newState):
            state = newState
            load()
            switch newState {
            case .ReadyToUse:
                break
            default:
                print("Weird state after migration: \(newState)")
            }
        case .failure(let err):
            handleError(err)
        }
    }
    
    func purge() {
        let lockbookDir = URL(fileURLWithPath: documenstDirectory).appendingPathComponent("lockbook.sled")
        if let _ = try? FileManager.default.removeItem(at: lockbookDir) {
            DispatchQueue.main.async {
                self.account = nil
                switch self.api.getState() {
                case .success(let db):
                    self.state = db
                case .failure(let err):
                    self.handleError(err)
                }
            }
        }
    }
    
    func handleError(_ error: AnyFfiError) {
        DispatchQueue.main.async {
            self.globalError = error
        }
    }
    
    func updateFiles() {
        print("Updating files!")
        if (account != nil) {
            switch api.getRoot() {
            case .success(let root):
                self.root = root
                switch api.listFiles() {
                case .success(let metas):
                    self.files = metas
                case .failure(let err):
                    handleError(err)
                }
            case .failure(let err):
                handleError(err)
            }
        }
    }
    
    init(documenstDirectory: String) {
        print("Initializing core...")
        self.documenstDirectory = documenstDirectory
        self.api = CoreApi(documentsDirectory: documenstDirectory)
        self.state = (try? self.api.getState().get())!
        self.account = (try? self.api.getAccount().get())
        updateFiles()
        
        syncChannel
            .debounce(for: .milliseconds(500), scheduler: RunLoop.main)
            .removeDuplicates(by: {
                switch ($0, $1) {
                case (.failure(let e1), .failure(let e2)):
                    return e1 == e2
                default:
                    return false
                }
            })
            .receive(on: RunLoop.main)
            .sink(receiveValue: { res in
                self.syncing = false
                switch res {
                case .success(_):
                    self.updateFiles()
                case .failure(let err):
                    self.handleError(err)
                }
            })
            .store(in: &cancellableSet)
    }
    
    init() {
        self.documenstDirectory = "<USING-FAKE-API>"
        self.api = FakeApi()
        self.state = .ReadyToUse
        self.account = Account(username: "testy", apiUrl: "ftp://lockbook.gov", keys: .empty)
        if case .success(let root) = api.getRoot(), case .success(let metas) = api.listFiles() {
            self.files = metas
            self.root = root
        }
    }
}

struct Message {
    let words: String
    let icon: String?
    let color: Color
}