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

// extract:
//   'unzip' → unzip <src> -d <native/<dest>>
//   'untar' → tar  -xzf <src> -C <native/<dest>>  (mkdir <dest> first)
//   'copy'  → copy  <src>     <native/<dest>/<name>>
const ASSETS = [
  { name: 'WalletCoreCommon.xcframework.zip', dest: 'ios',     extract: 'unzip' },
  { name: 'WalletCoreRs.xcframework.zip',     dest: 'ios',     extract: 'unzip' },
  { name: 'wallet-core.aar',                  dest: 'android', extract: 'copy'  },
  // v0.2.0+: Swift sources for the Expo Module podspec to compile alongside
  // its own .swift files. Untarred so that node_modules/@kyehyukahn/wallet-core/
  // native/ios/Sources/**/*.swift becomes a stable glob target.
  { name: 'swift-sources.tar.gz',             dest: 'ios',     extract: 'untar' },
  // v0.2.0+: Java protobuf-generated SigningInput types for the Expo Module
  // Kotlin code to build Ethereum.SigningInput / Solana.SigningInput.
  { name: 'wallet-core-proto.jar',            dest: 'android', extract: 'copy'  },
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

  // 3. lay out final native/ tree
  for (const a of ASSETS) {
    const baseTarget = path.join(NATIVE_DIR, a.dest);
    ensureDir(baseTarget);
    const staged = path.join(stageDir, a.name);
    switch (a.extract) {
      case 'unzip':
        unzipTo(staged, baseTarget);
        break;
      case 'untar': {
        // The Swift sources land at native/ios/Sources/ — keep this stable
        // so podspec source_files glob can hard-code the path.
        const sourcesDir = path.join(baseTarget, 'Sources');
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

  fs.writeFileSync(MARKER, `${new Date().toISOString()}\n`);
  log(`installed v${VERSION} under ${path.relative(process.cwd(), NATIVE_DIR)}/`);
}

main().catch((err) => die(err && err.message ? err.message : String(err)));
