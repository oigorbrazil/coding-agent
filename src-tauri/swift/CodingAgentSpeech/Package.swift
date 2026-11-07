// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "CodingAgentSpeech",
    platforms: [.macOS(.v10_15)],
    products: [
        .library(name: "CodingAgentSpeech", type: .static, targets: ["CodingAgentSpeech"]),
    ],
    targets: [
        .target(
            name: "CodingAgentSpeech",
            linkerSettings: [.linkedFramework("Speech"), .linkedFramework("AVFoundation")]
        ),
        .testTarget(name: "CodingAgentSpeechTests", dependencies: ["CodingAgentSpeech"]),
    ],
    swiftLanguageModes: [.v5]
)
