import { build } from 'vitepress'
import path from 'path'
import { fileURLToPath } from 'url'

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), 'src')

console.log('Building docs from', root)

build(root).then(
  () => process.exit(0),
  e => { console.error('Build failed:', e); process.exit(1) }
)
