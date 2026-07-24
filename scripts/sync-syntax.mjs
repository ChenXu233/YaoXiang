/**
 * 同步 TextMate Grammar 到 docs 目录
 *
 * 将 canonical source（vscode-extension/language-pack/syntaxes/）中的
 * yaoxiang.tmLanguage.json 同步到 VitePress 文档目录。
 *
 * 用法: node scripts/sync-syntax.mjs
 *       node scripts/sync-syntax.mjs --watch    # 启动文件监听
 *
 * Canonical source: vscode-extension/language-pack/syntaxes/yaoxiang.tmLanguage.json
 * Build artifact:   docs/src/.vitepress/syntaxes/yaoxiang.tmLanguage.json
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync, watchFile } from 'fs'
import { resolve, dirname } from 'path'
import { fileURLToPath } from 'url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = resolve(__dirname, '..')

const SOURCE = resolve(root, 'vscode-extension/language-pack/syntaxes/yaoxiang.tmLanguage.json')
const DEST   = resolve(root, 'docs/src/.vitepress/syntaxes/yaoxiang.tmLanguage.json')

const BUILD_ARTIFACT_COMMENT =
  '═══════════════════════════════════════════════════════════════ ' +
  '  BUILD ARTIFACT — 请勿直接编辑此文件' +
  '  ' +
  '  此文件由 scripts/sync-syntax.mjs 从 canonical source' +
  '  (vscode-extension/language-pack/syntaxes/) 自动同步生成。' +
  '  如需修改语法规则，请在 canonical source 中编辑后运行:' +
  '    node scripts/sync-syntax.mjs' +
  '═══════════════════════════════════════════════════════════════'

function sync() {
  if (!existsSync(SOURCE)) {
    console.error(`[sync-syntax] 错误: 未找到 canonical source: ${SOURCE}`)
    process.exit(1)
  }

  // 确保目标目录存在
  mkdirSync(dirname(DEST), { recursive: true })

  // 读取源文件并替换 $comment 为 build artifact 标记
  const source = readFileSync(SOURCE, 'utf-8')
  const artifact = source.replace(
    /"\$comment"\s*:\s*"[^"]*"/,
    () => `"$comment": "${BUILD_ARTIFACT_COMMENT.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`
  )

  writeFileSync(DEST, artifact, 'utf-8')
  console.log(`[sync-syntax] ✅ 已同步: ${SOURCE} → ${DEST}`)
}

// 主入口
const args = process.argv.slice(2)
if (args.includes('--watch')) {
  sync()
  console.log('[sync-syntax] 👀 正在监听 canonical source 变化...')
  watchFile(SOURCE, { interval: 1000 }, (curr, prev) => {
    if (curr.mtimeMs !== prev.mtimeMs) {
      console.log(`[sync-syntax] 🔄 检测到变更 (${new Date().toLocaleTimeString()})`)
      sync()
    }
  })
} else {
  sync()
}
