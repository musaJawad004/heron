import XCTest
@testable import Vigil

final class ModelsTests: XCTestCase {
    func testStatusParsing() {
        XCTAssertEqual(SessionStatus(rawStatus: "busy"), .busy)
        XCTAssertEqual(SessionStatus(rawStatus: "needs_input"), .needsInput)
        XCTAssertEqual(SessionStatus(rawStatus: nil), .unknown)
        XCTAssertTrue(SessionStatus.needsInput.needsAttention)
    }

    func testDisplayName() {
        let s = Session(id: "x", cwd: "/Users/me/proj", lastActiveAt: Date())
        XCTAssertEqual(s.displayName, "proj")
    }
}
