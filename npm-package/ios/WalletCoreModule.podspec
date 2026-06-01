require 'json'

package = JSON.parse(File.read(File.join(__dir__, '..', 'package.json')))

# The podspec lives at <pkg>/ios/, the postinstall-downloaded native artifacts
# (xcframeworks + Swift sources) live at <pkg>/native/ios/. CocoaPods resolves
# file patterns relative to the podspec directory, so reach the native tree
# with `../native/ios/...`. Absolute paths (via File.expand_path/__dir__) are
# rejected by CocoaPods file-pattern validation.

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

  # Native binaries downloaded by postinstall.
  # vendored_frameworks handles `..` parent-path traversal correctly, so
  # xcframeworks stay under <pkg>/native/ios/ (postinstall extraction target,
  # gitignored, regenerated per install).
  s.vendored_frameworks = [
    '../native/ios/WalletCoreCommon.xcframework',
    '../native/ios/WalletCoreRs.xcframework',
  ]

  # Compile the Expo Module's own .swift files alongside the wallet-core
  # Swift Sources tree (HDWallet, AnySigner, Ethereum+Proto.swift, plus the
  # full set of protoc-generated <Chain>.pb.swift struct definitions).
  #
  # CocoaPods source_files is asymmetric with vendored_frameworks: glob
  # patterns that traverse `..` (e.g. '../native/ios/Sources/**/*.swift')
  # silently match 0 files. Anchor the Swift tree inside the podspec dir
  # via postinstall extraction to <pkg>/ios/Sources/ instead.
  s.source_files = [
    '*.swift',
    'chains/*.swift',
    'Sources/**/*.swift',
  ]
end
