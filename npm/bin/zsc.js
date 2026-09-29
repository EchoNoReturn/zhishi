#!/usr/bin/env node
'use strict';

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

function findBinary() {
  if (process.env.ZHISHI_BIN) return process.env.ZHISHI_BIN;

  const plat = `${process.platform}-${process.arch}`;
  const exe = process.platform === 'win32' ? 'zsc.exe' : 'zsc';
  const vendor = path.join(__dirname, '..', 'vendor', plat, exe);
  if (fs.existsSync(vendor)) return vendor;

  // 未来：optionalDependencies（zhishi-win32-x64 等）
  try {
    return require.resolve(`zhishi-${plat}/${exe}`);
  } catch {
    /* fall through */
  }

  return exe; // 走 PATH
}

function findNodeModules() {
  if (process.env.ZHISHI_NODE_MODULES) return process.env.ZHISHI_NODE_MODULES;
  let dir = path.join(__dirname, '..');
  for (let i = 0; i < 6; i++) {
    const nm = path.join(dir, 'node_modules');
    if (fs.existsSync(path.join(nm, 'mathjax-full'))) return nm;
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return path.join(__dirname, '..', 'node_modules');
}

const bin = findBinary();
const env = {
  ...process.env,
  ZHISHI_NODE: process.env.ZHISHI_NODE || process.execPath,
  ZHISHI_NODE_MODULES: findNodeModules(),
};

const child = spawn(bin, process.argv.slice(2), {
  stdio: 'inherit',
  env,
  shell: false,
});

child.on('error', (err) => {
  if (err.code === 'ENOENT') {
    console.error(
      'error: zsc native binary not found.\n' +
        '  build one with:  cargo build --release && node npm/scripts/copy-binary.js\n' +
        '  or set ZHISHI_BIN to the binary path',
    );
    process.exit(1);
  }
  console.error(String(err));
  process.exit(1);
});

child.on('exit', (code, signal) => {
  if (signal) process.exit(1);
  process.exit(code == null ? 1 : code);
});
