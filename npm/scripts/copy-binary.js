#!/usr/bin/env node
'use strict';

// 把 cargo 构建出的 zsc 复制到 vendor/<platform>-<arch>/
const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const root = path.join(__dirname, '..', '..');
const plat = `${process.platform}-${process.arch}`;
const exe = process.platform === 'win32' ? 'zsc.exe' : 'zsc';

const candidates = [
  path.join(root, 'target', 'release', exe),
  path.join(root, 'target', 'debug', exe),
];

const src = candidates.find((p) => fs.existsSync(p));
if (!src) {
  console.error('zsc binary not found. Run: cargo build --release -p zhishi-cli');
  process.exit(1);
}

const destDir = path.join(root, 'npm', 'vendor', plat);
fs.mkdirSync(destDir, { recursive: true });
const dest = path.join(destDir, exe);
fs.copyFileSync(src, dest);

// Unix 需要可执行位
if (process.platform !== 'win32') {
  fs.chmodSync(dest, 0o755);
}

console.log(`copied ${src} -> ${dest}`);
