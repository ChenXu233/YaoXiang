/**
 * VitePress 文档构建脚本
 *
 * 构建流程:
 *   1. 同步 TextMate grammar (canonical source → docs 目录)
 *   2. 运行 VitePress 构建
 *   3. 复制 wasm 文件到输出目录
 */

import { build } from 'vitepress'
import path from 'path'
import { fileURLToPath } from 'url'
import { cpSync, existsSync, mkdirSync } from 'fs'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.join(__dirname, 'src')
const distDir = path.join(__dirname, 'src', '.vitepress', 'dist')
const wasmPublicDir = path.join(__dirname, 'src', '.vitepress', 'public', 'wasm')
const wasmDistDir = path.join(distDir, 'wasm')

// Step 1: 同步 TextMate grammar (canonical source → docs 目录)
console.log('Syncing TextMate grammar from canonical source...')
const { execSync } = await import('child_process')
const syncScript = path.join(__dirname, '..', 'scripts', 'sync-syntax.mjs')
execSync(`node "${syncScript}"`, { stdio: 'inherit', cwd: path.join(__dirname, '..') })

console.log('Building docs from', root)

// Step 2: 运行 VitePress 构建
build(root).then(
  () => {
    // Step 3: 复制 wasm 文件到输出目录
    if (existsSync(wasmPublicDir)) {
      mkdirSync(wasmDistDir, { recursive: true })
      cpSync(wasmPublicDir, wasmDistDir, { recursive: true })
      console.log('Copied wasm files to', wasmDistDir)
    }
    process.exit(0)
  },
  e => { console.error('Build failed:', e); process.exit(1) }
)
