/**
 * RFC 元数据迁移脚本
 *
 * 将现有 RFC 文件的元数据从引用块格式迁移到 YAML frontmatter 格式。
 *
 * 用法：node scripts/migrate-rfc-metadata.js [--dry-run]
 *
 * 选项：
 *   --dry-run  只显示变更，不实际修改文件
 */

import fs from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const RFC_DIR = path.join(__dirname, '..', 'src', 'design', 'rfc')

// RFC 分类目录
const CATEGORIES = ['draft', 'review', 'accepted', 'deprecated', 'rejected']

// 状态映射
const STATUS_MAP = {
  '草案': 'draft',
  '审核中': 'review',
  '已接受': 'accepted',
  '已废弃': 'deprecated',
  '已拒绝': 'rejected',
  '永久草案': 'draft',
  '正式': 'accepted'
}

/**
 * 解析引用块中的元数据
 */
function parseBlockquoteMetadata(content) {
  const metadata = {}

  // 匹配 > **状态**: xxx 格式
  const statusMatch = content.match(/^>\s*\*\*状态\*\*[:：]\s*(.+)$/m)
  if (statusMatch) {
    metadata.status = statusMatch[1].trim()
  }

  // 匹配 > **作者**: xxx 格式
  const authorMatch = content.match(/^>\s*\*\*作者\*\*[:：]\s*(.+)$/m)
  if (authorMatch) {
    metadata.author = authorMatch[1].trim()
  }

  // 匹配 > **创建日期**: xxx 格式
  const createdMatch = content.match(/^>\s*\*\*创建日期\*\*[:：]\s*(.+)$/m)
  if (createdMatch) {
    metadata.created = createdMatch[1].trim()
  }

  // 匹配 > **最后更新**: xxx 格式
  const updatedMatch = content.match(/^>\s*\*\*最后更新\*\*[:：]\s*(.+)$/m)
  if (updatedMatch) {
    metadata.updated = updatedMatch[1].trim()
  }

  return metadata
}

/**
 * 从一级标题提取标题
 */
function extractTitle(content) {
  const titleMatch = content.match(/^#\s+(.+)$/m)
  return titleMatch ? titleMatch[1].trim() : ''
}

/**
 * 检查是否已有 YAML frontmatter
 */
function hasYamlFrontmatter(content) {
  return /^---\n[\s\S]*?\n---/.test(content)
}

/**
 * 迁移单个文件
 */
function migrateFile(filePath, dryRun = false) {
  const content = fs.readFileSync(filePath, 'utf-8')
  const filename = path.basename(filePath)

  // 如果已有 YAML frontmatter，检查是否需要更新
  if (hasYamlFrontmatter(content)) {
    const yamlMatch = content.match(/^---\n([\s\S]*?)\n---/)
    if (yamlMatch) {
      const yamlContent = yamlMatch[1]
      // 检查是否已有 status, author, created 字段
      if (yamlContent.includes('status:') && yamlContent.includes('author:') && yamlContent.includes('created:')) {
        return { file: filename, action: 'skip', reason: '已有完整 YAML frontmatter' }
      }
    }
  }

  // 解析引用块中的元数据
  const metadata = parseBlockquoteMetadata(content)
  const title = extractTitle(content)

  if (!metadata.status && !metadata.author && !metadata.created) {
    return { file: filename, action: 'skip', reason: '未找到引用块元数据' }
  }

  // 构建新的 YAML frontmatter
  const newYaml = [
    '---',
    `title: "${title || filename.replace('.md', '')}"`,
    metadata.status ? `status: "${metadata.status}"` : 'status: "草案"',
    metadata.author ? `author: "${metadata.author}"` : 'author: "晨煦"',
    metadata.created ? `created: "${metadata.created}"` : 'created: "YYYY-MM-DD"',
    metadata.updated ? `updated: "${metadata.updated}"` : 'updated: "YYYY-MM-DD"',
    '---'
  ].join('\n')

  // 移除旧的引用块元数据
  let newContent = content

  // 如果已有 YAML frontmatter，替换它
  if (hasYamlFrontmatter(content)) {
    newContent = content.replace(/^---\n[\s\S]*?\n---/, newYaml)
  } else {
    // 在文件开头添加 YAML frontmatter
    newContent = newYaml + '\n\n' + content
  }

  // 移除引用块中的元数据行
  newContent = newContent.replace(/^>\s*\*\*状态\*\*[:：]\s*.+$/gm, '')
  newContent = newContent.replace(/^>\s*\*\*作者\*\*[:：]\s*.+$/gm, '')
  newContent = newContent.replace(/^>\s*\*\*创建日期\*\*[:：]\s*.+$/gm, '')
  newContent = newContent.replace(/^>\s*\*\*最后更新\*\*[:：]\s*.+$/gm, '')

  // 清理多余的空行
  newContent = newContent.replace(/\n{3,}/g, '\n\n')

  if (dryRun) {
    console.log(`\n--- ${filename} ---`)
    console.log('旧内容（前 20 行）：')
    console.log(content.split('\n').slice(0, 20).join('\n'))
    console.log('\n新内容（前 20 行）：')
    console.log(newContent.split('\n').slice(0, 20).join('\n'))
    return { file: filename, action: 'would-update', metadata }
  }

  fs.writeFileSync(filePath, newContent, 'utf-8')
  return { file: filename, action: 'updated', metadata }
}

/**
 * 主函数
 */
function main() {
  const dryRun = process.argv.includes('--dry-run')

  if (dryRun) {
    console.log('=== 干运行模式（不修改文件）===\n')
  }

  console.log('扫描 RFC 目录...')

  const results = []

  for (const category of CATEGORIES) {
    const categoryPath = path.join(RFC_DIR, category)

    if (!fs.existsSync(categoryPath)) continue

    const entries = fs.readdirSync(categoryPath)

    for (const entry of entries) {
      if (!entry.endsWith('.md')) continue
      if (entry === 'README.md') continue

      const filePath = path.join(categoryPath, entry)
      const result = migrateFile(filePath, dryRun)
      results.push({ ...result, category })
    }
  }

  // 打印统计
  console.log('\n=== 统计 ===')
  const updated = results.filter(r => r.action === 'updated' || r.action === 'would-update')
  const skipped = results.filter(r => r.action === 'skip')

  console.log(`更新: ${updated.length} 个文件`)
  console.log(`跳过: ${skipped.length} 个文件`)

  if (skipped.length > 0) {
    console.log('\n跳过的文件:')
    for (const r of skipped) {
      console.log(`  ${r.file}: ${r.reason}`)
    }
  }

  if (!dryRun && updated.length > 0) {
    console.log('\n迁移完成！')
    console.log('建议运行 `npm run generate:rfc-index` 重新生成索引。')
  }
}

main()
