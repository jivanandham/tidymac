import XCTest
@testable import TidyMacApp

/// Tests for the JSON → model layer and the treemap layout. These don't need
/// the Rust library to do any real work.
final class TidyMacBridgeTests: XCTestCase {

    func testScanResultParsesItemsAndEntries() {
        let dict: [String: Any] = [
            "scan_id": "abc",
            "profile": "developer",
            "duration_secs": 1.5,
            "total_reclaimable": UInt64(4096),
            "total_files": 3,
            "items": [[
                "id": 0,
                "name": "Xcode DerivedData",
                "category": "Dev: Xcode DerivedData",
                "size_bytes": UInt64(4096),
                "reclaimable_bytes": UInt64(1024),
                "entry_count": 1,
                "safety": "Safe",
                "cleanable": true,
                "review_only": false,
                "entries": [["path": "/Users/x/Library/Developer/Xcode/DerivedData/App-abc", "size_bytes": UInt64(4096)]],
            ]],
            "errors": [],
        ]
        let scan = ScanResult(from: dict)
        XCTAssertEqual(scan?.scanId, "abc")
        XCTAssertEqual(scan?.items.count, 1)
        XCTAssertEqual(scan?.items.first?.entries.first?.name, "App-abc")
        XCTAssertEqual(scan?.items.first?.hasSharedBlocks, false) // diff < 1 MB
    }

    func testCleanResultDistinguishesStagedFromFreed() {
        let result = CleanResult(from: [
            "mode": "soft_delete",
            "files_removed": 2,
            "bytes_freed": UInt64(0),
            "bytes_staged": UInt64(2048),
            "session_id": "2026-11-08T10-00-00-123-ab12",
            "skipped": [["path": "/x", "reason": "in use"]],
        ])
        XCTAssertEqual(result?.isSoftDelete, true)
        XCTAssertEqual(result?.bytesStaged, 2048)
        XCTAssertEqual(result?.bytesFreed, 0)
        XCTAssertEqual(result?.skipped.first?.reason, "in use")
        XCTAssertNil(result?.error)
    }

    func testCleanResultCarriesErrors() {
        let result = CleanResult(from: ["error": "Run a scan first"])
        XCTAssertEqual(result?.error, "Run a scan first")
    }

    func testAppIdentifierPrefersBundleId() {
        let app = AppInfo(from: ["name": "Slack", "bundle_id": "com.tinyspeck.slackmacgap", "path": "/Applications/Slack.app"])
        XCTAssertEqual(app?.identifier, "com.tinyspeck.slackmacgap")
        let noId = AppInfo(from: ["name": "Tool", "path": "/Applications/Tool.app"])
        XCTAssertEqual(noId?.identifier, "/Applications/Tool.app")
    }

    func testDupGroupNeedsTwoMembers() {
        XCTAssertNil(DupGroup(from: ["members": [["path": "/a", "size_bytes": UInt64(1)]]]))
        XCTAssertNotNil(DupGroup(from: ["members": [["path": "/a"], ["path": "/b"]]]))
    }

    func testTreemapFillsBoundsWithoutOverlap() {
        let nodes = [400, 300, 200, 100].enumerated().compactMap { i, size in
            DiskNode(from: ["name": "n\(i)", "path": "/n\(i)", "size": UInt64(size), "is_dir": true])
        }
        let bounds = CGRect(x: 0, y: 0, width: 200, height: 100)
        let tiles = Treemap.layout(nodes, in: bounds)
        XCTAssertEqual(tiles.count, 4)
        let area = tiles.reduce(0.0) { $0 + Double($1.rect.width * $1.rect.height) }
        XCTAssertEqual(area, 20_000, accuracy: 1)
        for i in 0..<tiles.count {
            for j in (i + 1)..<tiles.count {
                let overlap = tiles[i].rect.insetBy(dx: 0.01, dy: 0.01).intersects(tiles[j].rect.insetBy(dx: 0.01, dy: 0.01))
                XCTAssertFalse(overlap, "tiles \(i) and \(j) overlap")
            }
        }
    }
}
