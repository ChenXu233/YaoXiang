import { build } from 'vitepress'
import path from 'path'
import { fileURLToPath } from 'url'
import { cpSync, existsSync, mkdirSync } from 'fs'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.join(__dirname, 'src')
const distDir = path.join(__dirname, 'src', '.vitepress', 'dist')
const wasmPublicDir = path.join(__dirname, 'src', '.vitepress', 'public', 'wasm')
const wasmDistDir = path.join(distDir, 'wasm')

console.log('Building docs from', root)

build(root).then(
  () => {
    // Copy wasm files to dist (VitePress doesn't auto-copy from src/.vitepress/public/)
    if (existsSync(wasmPublicDir)) {
      mkdirSync(wasmDistDir, { recursive: true })
      cpSync(wasmPublicDir, wasmDistDir, { recursive: true })
      console.log('Copied wasm files to', wasmDistDir)
    }
    process.exit(0)
  },
  e => { console.error('Build failed:', e); process.exit(1) }
)
