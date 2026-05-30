'use strict';

const path = require('path');
const pkg = require('./package.json');

const NATIVE_DIR = path.join(__dirname, 'native');

module.exports = {
  /** Package version (== source git tag). */
  version: () => pkg.version,

  /** Absolute path to ios/WalletCoreCommon.xcframework. */
  iosCommonXcframework: () =>
    path.join(NATIVE_DIR, 'ios', 'WalletCoreCommon.xcframework'),

  /** Absolute path to ios/WalletCoreRs.xcframework. */
  iosRsXcframework: () =>
    path.join(NATIVE_DIR, 'ios', 'WalletCoreRs.xcframework'),

  /** Absolute path to android/wallet-core.aar. */
  androidAar: () =>
    path.join(NATIVE_DIR, 'android', 'wallet-core.aar'),

  /** Root of the unpacked native artifact tree. */
  nativeRoot: () => NATIVE_DIR,
};
