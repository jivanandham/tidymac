// swift-tools-version: 5.9
import PackageDescription

// The Rust core is linked as Libraries/libtidymac.dylib (built by scripts/build-ffi.sh).
//
// Runtime search paths (rpaths) are deliberately strict: the shipped app only
// loads the library from its own bundle (Contents/Frameworks). Relative paths
// such as "Libraries" or "../target/release" are resolved against the current
// working directory and would let a library planted there be loaded, so they
// are not used. Debug builds (`swift run`) additionally get the *absolute*
// path of this checkout's Libraries folder.
let libraryDir = "\(Context.packageDirectory)/Libraries"

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
                    "-L", libraryDir,
                    "-ltidymac",
                    "-Xlinker", "-rpath", "-Xlinker", "@executable_path/../Frameworks",
                ]),
                .unsafeFlags([
                    "-Xlinker", "-rpath", "-Xlinker", libraryDir,
                ], .when(configuration: .debug)),
            ]
        ),
        .testTarget(
            name: "TidyMacTests",
            dependencies: ["TidyMacApp"],
            path: "Tests/TidyMacTests",
            linkerSettings: [
                .unsafeFlags([
                    "-L", libraryDir,
                    "-ltidymac",
                    "-Xlinker", "-rpath", "-Xlinker", libraryDir,
                ]),
            ]
        ),
    ]
)
