import SwiftUI
import SwiftLockbookCore

struct FileBrowserView: View {
    @ObservedObject var core: Core
    let account: Account
    
    var body: some View {
        NavigationView {
            List {
                OutlineGroup(core.grouped, children: \.children) { meta in
                    if meta.meta.fileType == .Folder {
                        FileCell(meta: meta.meta)
                    } else {
                        NavigationLink(destination: Text("Destination")) {
                            FileCell(meta: meta.meta)
                        }
                    }
                }
                HStack {
                    Spacer()
                    Text("\(core.files.count) items")
                        .foregroundColor(.secondary)
                    Spacer()
                }
            }
            .listStyle(InsetGroupedListStyle())
            .navigationTitle("\(account.username)'s files")
            .toolbar(content: {
                HStack {
                    Button(action: self.core.sync) {
                        Image(systemName: "arrow.2.circlepath.circle.fill")
                    }
                    Button(action: self.core.purge) {
                        Image(systemName: "person.crop.circle.badge.xmark")
                    }
                }
            })
            
            Text("Pick a file!")
        }
    }
    
    func getFiles() -> [FileMetadata] {
        switch core.api.listFiles() {
        case .success(let files):
            return files
        case .failure(let err):
            core.displayError(error: err)
            return []
        }
    }
}

struct FileBrowserView_Previews: PreviewProvider {
    static var previews: some View {
        Group {
            FileBrowserView(core: Core(), account: Account(username: "test"))
                .ignoresSafeArea()
            FileBrowserView(core: Core(), account: Account(username: "test"))
                .ignoresSafeArea()
                .preferredColorScheme(.dark)
        }
    }
}

struct FileCell: View {
    let meta: FileMetadata
    
    var body: some View {
        VStack(alignment: .leading) {
            Text(meta.name)
            Label(intEpochToString(epoch: meta.contentVersion), systemImage: meta.fileType == .Folder ? "folder" : "doc")
                .font(.footnote)
                .foregroundColor(.secondary)
        }
    }
}