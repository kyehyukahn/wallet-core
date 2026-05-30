/**
 * Path resolver for native artifacts bundled with the kyehyukahn/wallet-core fork.
 *
 * The actual binaries are downloaded by `postinstall.js` from the matching
 * GitHub Release on install. These accessors return absolute paths under
 * `node_modules/@kyehyukahn/wallet-core/native/`.
 */

/** Package version (matches the source git tag, e.g. "0.1.0"). */
export function version(): string;

/** Absolute path to `ios/WalletCoreCommon.xcframework`. */
export function iosCommonXcframework(): string;

/** Absolute path to `ios/WalletCoreRs.xcframework`. */
export function iosRsXcframework(): string;

/** Absolute path to `android/wallet-core.aar`. */
export function androidAar(): string;

/** Root of the unpacked native artifact tree. */
export function nativeRoot(): string;
