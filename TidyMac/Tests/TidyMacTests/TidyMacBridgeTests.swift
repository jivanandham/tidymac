import XCTest
@testable import TidyMacApp

final class TidyMacBridgeTests: XCTestCase {
    
    func testBridgeInitialization() {
        let bridge = TidyMacBridge.shared
        XCTAssertNotNil(bridge, "Bridge should be a singleton and always available")
    }
    
    func testFFIStringConversion() {
        // This is a dummy test to ensure the bridge compiles and can be called
        // We can't easily run the actual FFI calls without the compiled dylib in the library search path
        // but we can verify the bridge logic around it if we expose it.
    }
    
    func testScanCancellation() {
        // Verify that cancelScan calls don't crash
        TidyMacBridge.shared.cancelScan()
        XCTAssertTrue(true)
    }
}
