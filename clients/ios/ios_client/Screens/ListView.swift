//
//  ListView.swift
//  ios_client
//
//  Created by Raayan Pillai on 4/11/20.
//  Copyright © 2020 Lockbook. All rights reserved.
//

import SwiftUI

struct ListView: View {
    @EnvironmentObject var coordinator: Coordinator
    
    var body: some View {
        NavigationView {
            List {
                ForEach(self.coordinator.files) { file in
                    FileRow(metadata: file)
                }
            }
            .navigationBarTitle("\(self.coordinator.username)'s Files")
            .navigationBarItems(
                leading: NavigationLink(destination: DebugView()) {
                    Image(systemName: "circle.grid.hex")
                },
                trailing: NavigationLink(destination: CreateFileView()) {
                    Image(systemName: "plus")
                }
            )
        }
    }
}

struct ListView_Previews: PreviewProvider {
    static var previews: some View {
        ListView().environmentObject(Coordinator())
    }
}
