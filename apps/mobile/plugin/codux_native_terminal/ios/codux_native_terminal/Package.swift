// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "codux_native_terminal",
    platforms: [
        .iOS(.v16),
        .macOS(.v13),
    ],
    products: [
        .library(name: "codux-native-terminal", targets: ["codux_native_terminal"]),
    ],
    dependencies: [
        .package(
            url: "https://github.com/Lakr233/libghostty-spm.git",
            exact: "1.2.3"
        ),
    ],
    targets: [
        .target(
            name: "codux_native_terminal",
            dependencies: [
                .product(name: "GhosttyTerminal", package: "libghostty-spm"),
                .product(name: "GhosttyTheme", package: "libghostty-spm"),
            ],
            path: "Sources/codux_native_terminal"
        ),
    ]
)
