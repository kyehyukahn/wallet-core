#!/usr/bin/env node
/**
 * Post-install hook for @kyehyukahn/wallet-core.
 *
 * Downloads the native artifacts (iOS xcframeworks + Android AAR) for the
 * matching version from the GitHub Releases of kyehyukahn/wallet-core,
 * verifies SHA-256 checksums, and unpacks them under ./native/.
 *
 * Skip conditions:
 *   - WALLET_CORE_SKIP_DOWNLOAD=1            : skip entirely (CI cache restore, etc.)
 *   - native/ already populated for this ver : skip with a notice
 *
 * Override:
 *   - WALLET_CORE_RELEASE_BASE_URL           : alternate Release URL prefix
 *                                              (default: GitHub Releases for this version)
 */

'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const https = require('https');
const { execFileSync } = require('child_process');

const pkg = require('./package.json');
const VERSION = pkg.version;
const PKG_DIR = __dirname;
const NATIVE_DIR = path.join(PKG_DIR, 'native');
const MARKER = path.join(NATIVE_DIR, `.installed-${VERSION}`);

const BASE_URL =
  process.env.WALLET_CORE_RELEASE_BASE_URL ||
  `https://github.com/kyehyukahn/wallet-core/releases/download/v${VERSION}`;

// Layout roots:
//   root: 'native' (default) → <pkg>/native/<dest>/
//   root: 'pkg'              → <pkg>/<dest>/
//
// extract:
//   'unzip' → unzip <src> -d <root>/<dest>
//   'untar' → tar  -xzf <src> -C <root>/<dest>  (mkdir <dest> first)
//   'copy'  → copy  <src>     <root>/<dest>/<name>
const ASSETS = [
  // v0.2.6+: xcframeworks land inside the podspec directory (<pkg>/ios/) too,
  // not under <pkg>/native/ios/. CocoaPods silently drops `vendored_frameworks`
  // entries that traverse `..` (the same asymmetry that bit `source_files`
  // in v0.2.3, which we'd assumed was source_files-only). Anchoring all
  // pod-consumed assets inside the podspec dir is the only reliable shape.
  { name: 'WalletCoreCommon.xcframework.zip', root: 'pkg', dest: 'ios',     extract: 'unzip' },
  { name: 'WalletCoreRs.xcframework.zip',     root: 'pkg', dest: 'ios',     extract: 'unzip' },
  // Android tools accept absolute paths from build.gradle, so the AAR /
  // proto.jar can stay under <pkg>/native/android/ with no glob-relative
  // gotcha. No `..` traversal involved in build.gradle's `files(...)` API.
  { name: 'wallet-core.aar',                  root: 'native', dest: 'android', extract: 'copy'  },
  { name: 'wallet-core-proto.jar',            root: 'native', dest: 'android', extract: 'copy'  },
  // v0.2.3+: Swift sources at <pkg>/ios/Sources/ (in-podspec-dir glob,
  // 'Sources/**/*.swift', same reason as above).
  { name: 'swift-sources.tar.gz',             root: 'pkg', dest: 'ios',     extract: 'untar' },
];
const SUMS_FILE = 'SHA256SUMS';

function log(msg) { console.log(`[@kyehyukahn/wallet-core] ${msg}`); }
function warn(msg) { console.warn(`[@kyehyukahn/wallet-core] ${msg}`); }
function die(msg) { console.error(`[@kyehyukahn/wallet-core] ${msg}`); process.exit(1); }

function downloadToFile(url, filePath) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(filePath);
    const get = (u, redirects = 0) => {
      https.get(u, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          if (redirects >= 5) return reject(new Error(`too many redirects: ${url}`));
          res.resume();
          return get(res.headers.location, redirects + 1);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode} for ${u}`));
        }
        res.pipe(file);
        file.on('finish', () => file.close((err) => err ? reject(err) : resolve()));
      }).on('error', reject);
    };
    get(url);
  });
}

async function downloadString(url) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    const get = (u, redirects = 0) => {
      https.get(u, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          if (redirects >= 5) return reject(new Error(`too many redirects: ${url}`));
          res.resume();
          return get(res.headers.location, redirects + 1);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode} for ${u}`));
        }
        res.on('data', (c) => chunks.push(c));
        res.on('end', () => resolve(Buffer.concat(chunks).toString('utf8')));
      }).on('error', reject);
    };
    get(url);
  });
}

function sha256OfFile(filePath) {
  const h = crypto.createHash('sha256');
  h.update(fs.readFileSync(filePath));
  return h.digest('hex');
}

function parseSums(text) {
  const map = {};
  for (const line of text.split(/\r?\n/)) {
    const m = line.trim().match(/^([0-9a-fA-F]{64})\s+\*?(.+)$/);
    if (m) map[m[2]] = m[1].toLowerCase();
  }
  return map;
}

function ensureDir(p) { fs.mkdirSync(p, { recursive: true }); }

function unzipTo(zip, destDir) {
  // unzip is part of macOS base + most Linux distros. The Release assets are produced by
  // macOS `ditto -c -k`, so the standard `unzip` handles them fine (Info.plist + slices).
  execFileSync('unzip', ['-q', '-o', zip, '-d', destDir], { stdio: 'inherit' });
}

function untarTo(tarball, destDir) {
  // BSD tar (macOS) and GNU tar (Linux) both support -xzf with -C destination.
  // The release-pack step uses `tar -czf` with `-C swift Sources`, so the
  // archive root is `Sources/`. We strip that prefix so files land directly
  // under destDir (which is already `native/ios/Sources` by convention).
  execFileSync('tar', ['-xzf', tarball, '--strip-components=1', '-C', destDir], { stdio: 'inherit' });
}

async function main() {
  if (process.env.WALLET_CORE_SKIP_DOWNLOAD === '1') {
    log('WALLET_CORE_SKIP_DOWNLOAD=1 — skipping native artifact download.');
    return;
  }
  if (fs.existsSync(MARKER)) {
    log(`native/ already populated for v${VERSION} — skipping.`);
    return;
  }

  log(`fetching native artifacts for v${VERSION} from ${BASE_URL}`);
  ensureDir(NATIVE_DIR);
  const stageDir = path.join(NATIVE_DIR, '.stage');
  fs.rmSync(stageDir, { recursive: true, force: true });
  ensureDir(stageDir);

  // 1. checksums
  const sumsText = await downloadString(`${BASE_URL}/${SUMS_FILE}`);
  const sums = parseSums(sumsText);

  // 2. download + verify each asset
  for (const a of ASSETS) {
    const expected = sums[a.name];
    if (!expected) die(`SHA256SUMS missing entry for ${a.name}`);
    const local = path.join(stageDir, a.name);
    log(`downloading ${a.name}`);
    await downloadToFile(`${BASE_URL}/${a.name}`, local);
    const actual = sha256OfFile(local);
    if (actual !== expected) {
      die(`checksum mismatch for ${a.name}\n  expected ${expected}\n  actual   ${actual}`);
    }
    log(`  ok (sha256 ${actual.slice(0, 16)}…)`);
  }

  // 3. lay out final tree
  for (const a of ASSETS) {
    const rootDir = a.root === 'pkg' ? PKG_DIR : NATIVE_DIR;
    const baseTarget = path.join(rootDir, a.dest);
    ensureDir(baseTarget);
    const staged = path.join(stageDir, a.name);
    switch (a.extract) {
      case 'unzip':
        unzipTo(staged, baseTarget);
        break;
      case 'untar': {
        // Swift sources land at <pkg>/ios/Sources/ — podspec source_files
        // globs against this path via 'Sources/**/*.swift'.
        const sourcesDir = path.join(baseTarget, 'Sources');
        // Wipe any previous extraction at the same destination to avoid
        // stale files surviving a downgrade or partial earlier install.
        fs.rmSync(sourcesDir, { recursive: true, force: true });
        ensureDir(sourcesDir);
        untarTo(staged, sourcesDir);
        break;
      }
      case 'copy':
        fs.copyFileSync(staged, path.join(baseTarget, a.name));
        break;
      default:
        die(`unknown extract mode for ${a.name}: ${a.extract}`);
    }
  }
  fs.rmSync(stageDir, { recursive: true, force: true });

  // 4. Android — unpack wallet-core.aar into JAR + jniLibs (v0.2.17+).
  //    AGP 8.x rejects `implementation files("…wallet-core.aar")` because
  //    `bundleDebugAar` evaluates `hasLocalAarDeps` on every android-library
  //    module, including this one. Our published AAR has only classes.jar +
  //    jni/<ABI>/*.so (verified: no res/, no aidl, stub manifest), so the
  //    container is unnecessary — we split it back into the two AGP-native
  //    inputs and consume them via files(…classes.jar) + jniLibs.srcDirs in
  //    android/build.gradle. Net effect: identical APK contents, AGP 8/9 safe.
  const androidDir = path.join(NATIVE_DIR, 'android');
  const aarPath = path.join(androidDir, 'wallet-core.aar');
  if (fs.existsSync(aarPath)) {
    const unpackDir = path.join(androidDir, '.aar-unpack');
    fs.rmSync(unpackDir, { recursive: true, force: true });
    ensureDir(unpackDir);
    execFileSync('unzip', ['-q', '-o', aarPath, '-d', unpackDir], { stdio: 'inherit' });

    const stagedClasses = path.join(unpackDir, 'classes.jar');
    if (!fs.existsSync(stagedClasses)) {
      die(`wallet-core.aar unpack: classes.jar missing inside the AAR.`);
    }
    fs.renameSync(stagedClasses, path.join(androidDir, 'wallet-core-classes.jar'));

    const stagedJni = path.join(unpackDir, 'jni');
    if (!fs.existsSync(stagedJni)) {
      die(`wallet-core.aar unpack: jni/ directory missing inside the AAR.`);
    }
    const jniLibsDir = path.join(androidDir, 'jniLibs');
    fs.rmSync(jniLibsDir, { recursive: true, force: true });
    fs.renameSync(stagedJni, jniLibsDir);

    fs.rmSync(unpackDir, { recursive: true, force: true });
    log(`unpacked wallet-core.aar → wallet-core-classes.jar + jniLibs/`);
  }

  // 5. sanity guard: the Swift symbol surface that chains/EvmSigning.swift
  //    depends on must be present after extraction. Catches a class of
  //    repack regressions (empty tarball, wrong destination, bad strip)
  //    that would otherwise only surface as opaque "cannot find type
  //    TW_*_Proto_*" errors during a consumer's xcodebuild.
  const pbSentinel = path.join(PKG_DIR, 'ios', 'Sources', 'Generated', 'Protobuf', 'Ethereum.pb.swift');
  if (!fs.existsSync(pbSentinel)) {
    die(
      `postinstall sanity check failed: ${path.relative(PKG_DIR, pbSentinel)} not present after extraction.\n` +
      `The swift-sources.tar.gz published with this version is missing required\n` +
      `protoc-generated files. Refuse to leave the package in a half-built state.`
    );
  }

  fs.writeFileSync(MARKER, `${new Date().toISOString()}\n`);
  log(`installed v${VERSION} under ${path.relative(process.cwd(), NATIVE_DIR)}/`);
}

main().catch((err) => die(err && err.message ? err.message : String(err)));
