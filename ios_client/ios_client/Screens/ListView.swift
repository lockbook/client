//
//  ListView.swift
//  ios_client
//
//  Created by Raayan Pillai on 4/11/20.
//  Copyright © 2020 Lockbook. All rights reserved.
//

import SwiftUI

struct ListView: View {
    var lockbookApi: LockbookApi
    var username: String
    @State private var files: [FileMetadata]
    @EnvironmentObject var screenCoordinator: ScreenCoordinator

    var body: some View {
        VStack {
            NavigationView {
                List {
                    ForEach(files) { file in
                        FileRow(lockbookApi: self.lockbookApi, metadata: file)
                    }
                }
                .navigationBarTitle("\(self.username)'s Files")
                .navigationBarItems(trailing:
                    NavigationLink(destination: CreateFileView(lockbookApi: self.lockbookApi)) {
                        Image(systemName: "plus")
                    }
                )
                
            }
            MonokaiButton(text: "Reload Files")
                .onTapGesture {
                    self.files = self.lockbookApi.updateMetadata()
                }
        }
    }
    
    init(lockbookApi: LockbookApi) {
        self.lockbookApi = lockbookApi
        self._files = State(initialValue: lockbookApi.updateMetadata())
        if let username = lockbookApi.getAccount() {
            self.username = username
        } else {
            self.username = "<USERNAME>"
        }
    }
}

struct ListView_Previews: PreviewProvider {
    static var previews: some View {
        ListView(lockbookApi: FakeApi()).environmentObject(ScreenCoordinator())
    }
}
