/**
 * RFC 索引自动生成脚本
 *
 * 扫描 docs/src/design/rfc/ 目录，解析每个 RFC 文件的 frontmatter，
 * 自动生成 index.md 索引文件。
 *
 * 用法：node scripts/generate-rfc-index.js
 */

import fs from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const RFC_DIR = path.join(__dirname, '..', 'src', 'design', 'rfc')

// RFC 分类目录
const CATEGORIES = {
  draft: '草案RFC',
  review: '审核中RFC',
  accepted: '已接受RFC',
  deprecated: '已废弃RFC',
  rejected: '已拒绝RFC'
}

/**
 * 解析 Markdown 文件的元数据
 *
 * 支持两种格式：
 * 1. YAML frontmatter（推荐）
 * 2. 引用块中的元数据（兼容旧格式）
 */
function parseFrontmatter(content) {
  const frontmatter = {}

  // 优先解析 YAML frontmatter
  const yamlMatch = content.match(/^---\n([\s\S]*?)\n---/)
  if (yamlMatch) {
    const lines = yamlMatch[1].split('\n')

    for (const line of lines) {
      const colonIndex = line.indexOf(':')
      if (colonIndex === -1) continue

      const key = line.slice(0, colonIndex).trim()
      let value = line.slice(colonIndex + 1).trim()

      // 移除引号
      if ((value.startsWith('"') && value.endsWith('"')) ||
          (value.startsWith("'") && value.endsWith("'"))) {
        value = value.slice(1, -1)
      }

      // 映射字段名
      const fieldMap = {
        'title': 'title',
        'status': 'status',
        'author': 'author',
        'created': 'created',
        'updated': 'updated',
        '创建日期': 'created',
        '最后更新': 'updated',
        '状态': 'status',
        '作者': 'author'
      }

      const mappedKey = fieldMap[key] || key
      frontmatter[mappedKey] = value
    }
  }

  // 如果 YAML frontmatter 中没有完整信息，尝试解析引用块
  if (!frontmatter.status || !frontmatter.author || !frontmatter.created) {
    const blockquoteMatch = content.match(/^>\s*\*\*(.+?)\*\*[:：]\s*(.+)$/gm)
    if (blockquoteMatch) {
      for (const line of blockquoteMatch) {
        const match = line.match(/^>\s*\*\*(.+?)\*\*[:：]\s*(.+)$/)
        if (match) {
          const key = match[1].trim()
          const value = match[2].trim()

          const fieldMap = {
            '状态': 'status',
            '作者': 'author',
            '创建日期': 'created',
            '最后更新': 'updated'
          }

          const mappedKey = fieldMap[key] || key
          if (!frontmatter[mappedKey]) {
            frontmatter[mappedKey] = value
          }
        }
      }
    }
  }

  // 尝试解析一级标题作为标题
  if (!frontmatter.title) {
    const titleMatch = content.match(/^#\s+(.+)$/m)
    if (titleMatch) {
      frontmatter.title = titleMatch[1].trim()
    }
  }

  return frontmatter
}

/**
 * 从文件名提取 RFC 编号
 *
 * 支持格式：
 * - `014-package-manager.md` → { number: 14, sub: null }
 * - `014a-registry-protocol.md` → { number: 14, sub: 'a' }
 */
function extractRfcNumber(filename) {
  const match = filename.match(/^(\d+)([a-z]?)-/)
  if (!match) return null
  return {
    number: parseInt(match[1], 10),
    sub: match[2] || null
  }
}

/**
 * 格式化 RFC 编号为显示字符串
 */
function formatRfcNumber(rfcInfo) {
  if (!rfcInfo) return ''
  const base = `RFC-${String(rfcInfo.number).padStart(3, '0')}`
  return rfcInfo.sub ? `${base}${rfcInfo.sub}` : base
}

/**
 * 扫描目录，获取所有 RFC 文件
 */
function scanRfcFiles(dir, category) {
  const files = []
  const categoryPath = path.join(dir, category)

  if (!fs.existsSync(categoryPath)) return files

  const entries = fs.readdirSync(categoryPath)

  for (const entry of entries) {
    if (!entry.endsWith('.md')) continue
    if (entry === 'README.md') continue

    const filePath = path.join(categoryPath, entry)
    const content = fs.readFileSync(filePath, 'utf-8')
    const frontmatter = parseFrontmatter(content)
    const rfcInfo = extractRfcNumber(entry)

    if (!rfcInfo) continue

    // 从内容中提取标题（如果有 frontmatter 则使用，否则从一级标题提取）
    let title = frontmatter.title || ''
    if (!title) {
      const titleMatch = content.match(/^#\s+(.+)$/m)
      title = titleMatch ? titleMatch[1] : entry
    }

    // 提取作者
    const author = frontmatter.author || '晨煦'

    // 提取创建日期
    const created = frontmatter['创建日期'] || frontmatter.created || ''

    // 提取状态
    let status = frontmatter['状态'] || frontmatter.status || ''
    if (!status) {
      // 从目录推断状态
      const statusMap = {
        draft: '草案',
        review: '审核中',
        accepted: '已接受',
        deprecated: '已废弃',
        rejected: '已拒绝'
      }
      status = statusMap[category] || ''
    }

    // 提取 group 字段
    const group = frontmatter.group || null

    files.push({
      rfcInfo,
      title,
      author,
      created,
      status,
      group,
      filename: entry,
      category,
      relativePath: `./${category}/${entry}`
    })
  }

  return files
}

/**
 * 生成索引 Markdown
 */
function generateIndex(allRfcs) {
  const lines = []

  lines.push('---')
  lines.push('title: "RFC 索引"')
  lines.push('---')
  lines.push('')
  lines.push('# YaoXiang RFC（请求评议）索引')
  lines.push('')
  lines.push('> RFC（Request for Comments）是YaoXiang语言特性设计提案的正式提交格式。')
  lines.push('')
  lines.push('## 目录')
  lines.push('')
  lines.push('- [模板](#模板)')
  lines.push('- [草案RFC](#草案rfc)')
  lines.push('- [审核中RFC](#审核中rfc)')
  lines.push('- [已接受RFC](#已接受rfc)')
  lines.push('- [已废弃RFC](#已废弃rfc)')
  lines.push('- [已拒绝RFC](#已拒绝rfc)')
  lines.push('')
  lines.push('---')
  lines.push('')

  // 模板
  lines.push('## 模板')
  lines.push('')
  lines.push('| 文件 | 说明 |')
  lines.push('|------|------|')
  lines.push('| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC标准模板 |')
  lines.push('| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完整示例（模式匹配增强） |')
  lines.push('')
  lines.push('---')
  lines.push('')

  // 预计算：哪些子 RFC 属于其他分类的父 RFC（需要从原分类中移除）
  const groupedSubIds = new Set()
  for (const rfc of allRfcs) {
    if (rfc.rfcInfo.sub || !rfc.group) continue
    // 这是一个有 group 字段的父 RFC（不应该发生，但防御性编程）
  }
  for (const rfc of allRfcs) {
    if (!rfc.rfcInfo.sub || !rfc.group) continue
    // 找到这个子 RFC 的父 RFC
    const parent = allRfcs.find(p => !p.rfcInfo.sub && `rfc-${String(p.rfcInfo.number).padStart(3, '0')}` === rfc.group)
    if (parent && parent.category !== rfc.category) {
      groupedSubIds.add(`${rfc.rfcInfo.number}-${rfc.rfcInfo.sub}`)
    }
  }

  // 各分类
  for (const [category, categoryName] of Object.entries(CATEGORIES)) {
    lines.push(`## ${categoryName}`)
    lines.push('')

    const rfcs = allRfcs.filter(rfc => rfc.category === category)

    if (rfcs.length === 0) {
      lines.push('| 编号 | 标题 | 作者 | 创建日期 | 状态 |')
      lines.push('|------|------|------|----------|------|')
      lines.push('| （暂无） | | | | |')
    } else {
      lines.push('| 编号 | 标题 | 作者 | 创建日期 | 状态 |')
      lines.push('|------|------|------|----------|------|')

      // 分离父 RFC 和子 RFC
      const parents = rfcs.filter(rfc => !rfc.rfcInfo.sub)
      const subs = rfcs.filter(rfc => rfc.rfcInfo.sub)

      // 按编号排序
      parents.sort((a, b) => a.rfcInfo.number - b.rfcInfo.number)

      for (const rfc of parents) {
        const number = formatRfcNumber(rfc.rfcInfo)
        const title = `[${rfc.title}](${rfc.relativePath})`
        lines.push(`| ${number} | ${title} | ${rfc.author} | ${rfc.created} | ${rfc.status} |`)

        // 查找属于此父 RFC 的子 RFC（跨所有分类）
        const parentGroup = `rfc-${String(rfc.rfcInfo.number).padStart(3, '0')}`
        const children = allRfcs
          .filter(s => s.rfcInfo.sub && s.group === parentGroup)
          .sort((a, b) => a.rfcInfo.sub.localeCompare(b.rfcInfo.sub))

        for (const child of children) {
          const childNumber = formatRfcNumber(child.rfcInfo)
          const childTitle = `[${child.title}](${child.relativePath})`
          const childStatus = child.category !== category
            ? CATEGORIES[child.category]
            : child.status
          lines.push(`| ↳ ${childNumber} | ${childTitle} | ${child.author} | ${child.created} | ${childStatus} |`)
          groupedSubIds.add(`${child.rfcInfo.number}-${child.rfcInfo.sub}`)
        }
      }

      // 本分类中未被归组的子 RFC
      const orphanSubs = subs.filter(s => !groupedSubIds.has(`${s.rfcInfo.number}-${s.rfcInfo.sub}`))
      if (orphanSubs.length > 0) {
        orphanSubs.sort((a, b) => {
          const numDiff = a.rfcInfo.number - b.rfcInfo.number
          if (numDiff !== 0) return numDiff
          return a.rfcInfo.sub.localeCompare(b.rfcInfo.sub)
        })
        for (const rfc of orphanSubs) {
          const number = formatRfcNumber(rfc.rfcInfo)
          const title = `[${rfc.title}](${rfc.relativePath})`
          lines.push(`| ${number} | ${title} | ${rfc.author} | ${rfc.created} | ${rfc.status} |`)
        }
      }
    }

    lines.push('')
    lines.push('---')
    lines.push('')
  }

  // RFC 生命周期
  lines.push('## RFC生命周期')
  lines.push('')
  lines.push('```')
  lines.push('草案 → 审核中 → 已接受 → 已废弃（被取代）')
  lines.push('                  ↓')
  lines.push('               已拒绝（不通过）')
  lines.push('```')
  lines.push('')

  // 状态说明
  lines.push('### 状态说明')
  lines.push('')
  lines.push('| 状态 | 位置 | 说明 |')
  lines.push('|------|------|------|')
  lines.push('| **草案** | `rfc/draft/` | 作者草稿，等待提交审核 |')
  lines.push('| **审核中** | `rfc/review/` | 开放社区讨论和反馈 |')
  lines.push('| **已接受** | `rfc/accepted/` | 成为正式设计文档，进入实现阶段 |')
  lines.push('| **已废弃** | `rfc/deprecated/` | 曾被接受，被新设计取代 |')
  lines.push('| **已拒绝** | `rfc/rejected/` | 被拒绝的RFC文档 |')
  lines.push('')
  lines.push('---')
  lines.push('')

  // 提交 RFC
  lines.push('## 提交RFC')
  lines.push('')
  lines.push('1. 阅读 [RFC_TEMPLATE.md](RFC_TEMPLATE.md) 了解格式要求')
  lines.push('2. 参考 [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) 学习写法')
  lines.push('3. 创建新文件，命名为 `序号-描述性标题.md`')
  lines.push('4. 将文件放入 `docs/reference/rfc/draft/` 目录')
  lines.push('5. 更新本索引文件，添加新RFC条目')
  lines.push('6. 提交PR进入审核流程')
  lines.push('')
  lines.push('---')
  lines.push('')

  // 贡献指南
  lines.push('## 贡献指南')
  lines.push('')
  lines.push('请参阅 [CONTRIBUTING.md](../../../../CONTRIBUTING.md) 了解贡献指南。')

  return lines.join('\n')
}

/**
 * 主函数
 */
function main() {
  console.log('扫描 RFC 目录...')

  const allRfcs = []

  for (const category of Object.keys(CATEGORIES)) {
    const rfcs = scanRfcFiles(RFC_DIR, category)
    allRfcs.push(...rfcs)
    console.log(`  ${category}: ${rfcs.length} 个 RFC`)
  }

  console.log(`\n共发现 ${allRfcs.length} 个 RFC`)

  // 生成索引
  const indexContent = generateIndex(allRfcs)
  const indexPath = path.join(RFC_DIR, 'index.md')

  fs.writeFileSync(indexPath, indexContent, 'utf-8')
  console.log(`\n已生成索引文件: ${indexPath}`)
}

main()
