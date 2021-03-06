import XCTest
@testable import SwiftLockbookCore

class UsageTests: SLCTest {
    func testSimple() throws {
        let resultCreateAccount = try core.api.createAccount(username: randomUsername(), apiLocation: systemApiLocation())
        
        assertSuccess(resultCreateAccount)
        
        let resultUsage = core.api.getUsage()
        
        assertSuccess(resultUsage) { usages in
            usages.serverUsage.exact == 0
        }
    }
}
