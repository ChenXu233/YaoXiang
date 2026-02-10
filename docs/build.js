import { build } from 'vitepress'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
// 当前目录即为 docs 根目录
const root = __dirname 

console.log('Building docs...')

try {
  await build(root)
  console.log('Build complete. Forcing exit.')
  process.exit(0)
} catch (e) {
  console.error('Build failed:', e)
  process.exit(1)
}
