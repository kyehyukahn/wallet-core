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
  s.vendored_frameworks = [
    '../native/ios/WalletCoreCommon.xcframework',
    '../native/ios/WalletCoreRs.xcframework',
  ]

  # Compile the Expo Module's own .swift files alongside the wallet-core
  # Swift Sources tree (HDWallet, AnySigner, Ethereum+Proto.swift, etc.)
  # so chain signing code can use them directly without a separate Swift
  # package boundary.
  s.source_files = [
    '*.swift',
    'chains/*.swift',
    '../native/ios/Sources/**/*.swift',
  ]
end
