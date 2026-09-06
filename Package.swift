// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "Vigil",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "Vigil", targets: ["Vigil"]),
    ],
    targets: [
        .executableTarget(
            name: "Vigil",
            path: "Sources/Vigil",
            resources: [.copy("Resources")],
            swiftSettings: [
                .enableUpcomingFeature("StrictConcurrency"),
            ]
        ),
        .testTarget(
            name: "VigilTests",
            dependencies: ["Vigil"],
            path: "Tests/VigilTests"
        ),
    ]
)
