require 'json'

package = JSON.parse(File.read(File.join(__dir__, '..', 'package.json')))

# The podspec lives at <pkg>/ios/. All postinstall-downloaded iOS artifacts
# (xcframeworks + Swift sources) land inside this same directory in v0.2.6+.
# Anything CocoaPods consumes via file globs MUST be inside the podspec
# directory — both `source_files` and `vendored_frameworks` silently drop
# patterns that traverse `..`. (This had bitten us as B4 for source_files in
# v0.2.2 and as B7 for vendored_frameworks in v0.2.5.)
#
# Android artifacts (AAR / proto.jar) stay under <pkg>/native/android/
# because Gradle's `files(...)` accepts absolute paths and has no glob-
# relative path-traversal asymmetry.

Pod::Spec.new do |s|
  s.name             = 'WalletCoreModule'
  s.version          = package['version']
  s.summary          = 'Expo Module wrapping kyehyukahn/wallet-core for React Native.'
  s.description      = package['description']
  s.author           = ''
  s.homepage         = 'https://github.com/kyehyukahn/wallet-core'
  s.license          = { :type => package['license'] }
  s.platforms        = { :ios => '15.1' }
  s.source           = { :git => '' }
  s.static_framework = true
  s.swift_version    = '5.9'

  s.dependency 'ExpoModulesCore'
  # SwiftProtobuf is required by Sources/Generated/Protobuf/*.swift which
  # define the chain SigningInput / SigningOutput messages. Pinned to a
  # 1.x major to match what the upstream wallet-core Package.swift uses.
  s.dependency 'SwiftProtobuf', '~> 1.27'

  # In-podspec-dir paths so CocoaPods actually picks them up.
  s.vendored_frameworks = [
    'WalletCoreCommon.xcframework',
    'WalletCoreRs.xcframework',
  ]

  s.source_files = [
    '*.swift',
    'chains/*.swift',
    'Sources/**/*.swift',
  ]
end
