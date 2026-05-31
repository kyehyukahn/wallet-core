require 'json'

package = JSON.parse(File.read(File.join(__dir__, '..', 'package.json')))

# Resolve where this package lives on disk. When consumed via npm + autolinking
# the file lies under node_modules/@kyehyukahn/wallet-core/ios/, and the native
# binaries + Swift sources are under ../native/ios/ (postinstall output).
package_root = File.expand_path('..', __dir__)
native_ios   = File.join(package_root, 'native', 'ios')

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
    File.join(native_ios, 'WalletCoreCommon.xcframework'),
    File.join(native_ios, 'WalletCoreRs.xcframework'),
  ]

  # Compile the Expo Module's own .swift files alongside the wallet-core
  # Swift Sources tree (HDWallet, AnySigner, Ethereum+Proto.swift, etc.)
  # so chain signing code can use them directly without a separate Swift
  # package boundary.
  s.source_files = [
    '*.swift',
    'chains/*.swift',
    File.join(native_ios, 'Sources', '**', '*.swift'),
  ]
end
