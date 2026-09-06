// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "Heron",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "Heron", targets: ["Heron"]),
    ],
    targets: [
        .executableTarget(
            name: "Heron",
            path: "Sources/Heron",
            resources: [.copy("Resources")],
            swiftSettings: [
                .enableUpcomingFeature("StrictConcurrency"),
            ]
        ),
        .testTarget(
            name: "HeronTests",
            dependencies: ["Heron"],
            path: "Tests/HeronTests"
        ),
    ]
)
