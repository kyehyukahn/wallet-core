# @kyehyukahn/wallet-core

Native artifact bundle for the `kyehyukahn/wallet-core` fork (a TrustWallet
wallet-core fork that adds Bitcoin SPV and the build wiring needed by the
cirqle React Native app).

This package is intentionally tiny. On install, `postinstall.js` downloads
the matching GitHub Release for this package version and unpacks the
native artifacts into `node_modules/@kyehyukahn/wallet-core/native/`.

## What you get

```
native/
├── ios/
│   ├── WalletCoreCommon.xcframework/    (~264 MB, 4 slices)
│   └── WalletCoreRs.xcframework/        (~97 MB, 4 slices)
└── android/
    └── wallet-core.aar                  (~29 MB, 4 ABIs)
```

Total download per install: ~390 MB. The package itself ships only a
postinstall script and a few KB of JavaScript path resolvers.

## Installation

GitHub Packages npm registry requires authentication for **every** install
(unlike npmjs.org), regardless of package visibility. Both lines below are
required in the consumer project's `.npmrc`:

```bash
# Configure registry + auth (project root .npmrc)
cat >> .npmrc <<EOF
@kyehyukahn:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=\${GITHUB_TOKEN}
EOF

# Export a token with at least `read:packages` scope
export GITHUB_TOKEN=$(gh auth token)   # or use a Personal Access Token

# Install. The postinstall hook downloads native artifacts.
pnpm add @kyehyukahn/wallet-core@^0.1.0
# or: npm install / yarn add
```

For CI, `secrets.GITHUB_TOKEN` is sufficient (already has `read:packages`).

Note: the Release assets themselves (`.zip`, `.aar`, `SHA256SUMS`) are
hosted on a public repo and are downloaded anonymously — only the npm
registry call needs the token. Do **not** commit `.npmrc` with a literal
token; use the `${GITHUB_TOKEN}` indirection or add `.npmrc` to
`.gitignore`.

To skip the download (e.g., when restoring from CI cache):
```bash
WALLET_CORE_SKIP_DOWNLOAD=1 pnpm install
```

To pull artifacts from a non-default location:
```bash
WALLET_CORE_RELEASE_BASE_URL=https://example.com/mirrors/wallet-core/v0.1.0 pnpm install
```

## Usage

```js
const wc = require('@kyehyukahn/wallet-core');

wc.version();                  // "0.1.0"
wc.iosCommonXcframework();     // /abs/.../ios/WalletCoreCommon.xcframework
wc.iosRsXcframework();         // /abs/.../ios/WalletCoreRs.xcframework
wc.androidAar();               // /abs/.../android/wallet-core.aar
wc.nativeRoot();               // /abs/.../native
```

Intended consumer: an Expo / React Native config plugin (or a Gradle /
CocoaPods integration) that takes these paths and wires them into the
native build.

## Security note

The postinstall script verifies every downloaded asset against the
`SHA256SUMS` published in the same Release before unpacking. A mismatch
aborts the install.

## License

MIT (same as upstream trustwallet/wallet-core).
