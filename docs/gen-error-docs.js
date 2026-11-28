/**
 * 错误码参考页的构建期生成调用器。
 *
 * 错误码参考页（docs/src/reference/error-code/、warning-code/warning-codes.md、
 * package/error-codes.md，以及 en/ 下的英文镜像）是**构建期产物**：由
 * scripts/docs/gen-error-code-docs.py 从码表注册表与 locales/*.json 生成，
 * 不入库（见根 .gitignore 与 docs/src/dev/HOWTO.md）。
 *
 * 文档站构建与本地预览都先跑这一步——站点上的错误码页因此永远与码表一致，
 * 不需要「事后对账」式的门禁去盯入库快照。
 *
 * 用法：node docs/gen-error-docs.js
 */

import { execFileSync } from 'child_process'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.join(__dirname, '..')
const script = path.join(repoRoot, 'scripts', 'docs', 'gen-error-code-docs.py')

/** 找可用的 python 解释器（Windows 上是 python，CI 的 ubuntu 上是 python3）。 */
function pythonBin() {
  const candidates =
    process.platform === 'win32' ? ['python', 'python3'] : ['python3', 'python']
  for (const bin of candidates) {
    try {
      execFileSync(bin, ['--version'], { stdio: 'ignore' })
      return bin
    } catch {
      // 试下一个候选
    }
  }
  throw new Error(
    '未找到 Python 3 解释器：错误码参考页生成需要 python3（CI 用 python3，本地用 python）'
  )
}

const bin = pythonBin()
for (const lang of ['zh', 'en']) {
  execFileSync(bin, [script, '--lang', lang], { stdio: 'inherit', cwd: repoRoot })
}
