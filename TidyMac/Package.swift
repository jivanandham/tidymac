// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "TidyMacApp",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(name: "TidyMacApp", targets: ["TidyMacApp"]),
    ],
    targets: [
        .systemLibrary(
            name: "TidyMacFFI",
            path: "Libraries",
            pkgConfig: nil,
            providers: nil
        ),
        .executableTarget(
            name: "TidyMacApp",
            dependencies: ["TidyMacFFI"],
            path: "Sources",
            linkerSettings: [
                .unsafeFlags([
                    "-L", "../target/release",
                    "-L", "../target/debug",
                    "-ltidymac",
                    "-Xlinker", "-rpath", "-Xlinker", "@executable_path/../lib",
                    "-Xlinker", "-rpath", "-Xlinker", "../target/release",
                    "-Xlinker", "-rpath", "-Xlinker", "../target/debug",
                ]),
            ]
        ),
        .testTarget(
            name: "TidyMacTests",
            dependencies: ["TidyMacApp"],
            path: "Tests/TidyMacTests"
        ),
    ]
)
