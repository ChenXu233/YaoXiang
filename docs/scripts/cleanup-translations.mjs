import fs from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const docsDir = path.resolve(__dirname, '..')
const srcDir = path.join(docsDir, 'src')

// Read target languages from vpi18n.config.json
const configPath = path.join(docsDir, 'vpi18n.config.json')
let targets
try {
  const config = JSON.parse(fs.readFileSync(configPath, 'utf-8'))
  targets = config.target.split(',').map(s => s.trim())
} catch {
  console.error('Failed to read vpi18n.config.json')
  process.exit(1)
}

function walkDir(dir) {
  let files = []
  try {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const fp = path.join(dir, entry.name)
      if (entry.isDirectory()) files.push(...walkDir(fp))
      else if (entry.name.endsWith('.md')) files.push(fp)
    }
  } catch { /* directory doesn't exist */ }
  return files
}

// Read cache
const cachePath = path.join(srcDir, '.i18n-cache.json')
let cache = {}
try { cache = JSON.parse(fs.readFileSync(cachePath, 'utf-8')) } catch { /* no cache yet */ }

let changed = false

for (const lang of targets) {
  const langDir = path.join(srcDir, lang)
  if (!fs.existsSync(langDir)) continue

  for (const tf of walkDir(langDir)) {
    const relPath = path.relative(langDir, tf)
    const sourceFile = path.join(srcDir, relPath)

    if (!fs.existsSync(sourceFile)) {
      console.log(`Remove orphan: ${path.relative(docsDir, tf)}`)
      fs.unlinkSync(tf)
      changed = true

      // Clean up cache entry
      const cacheKey = 'src/' + relPath.replace(/\\/g, '/') + ':' + lang
      delete cache[cacheKey]
    }
  }
}

// Remove empty language subdirectories
for (const lang of targets) {
  const langDir = path.join(srcDir, lang)
  if (!fs.existsSync(langDir)) continue

  const removeEmptyDirs = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      if (entry.isDirectory()) {
        const fp = path.join(dir, entry.name)
        removeEmptyDirs(fp)
      }
    }
    try {
      if (fs.readdirSync(dir).length === 0) {
        console.log(`Remove empty dir: ${path.relative(docsDir, dir)}`)
        fs.rmdirSync(dir)
      }
    } catch { /* not empty or can't read */ }
  }
  removeEmptyDirs(langDir)
}

if (changed) {
  fs.writeFileSync(cachePath, JSON.stringify(cache, null, 2) + '\n')
} else {
  console.log('No orphaned translations found.')
}
