#!/usr/bin/env python3
"""
commit-msg format checker for pre-commit framework.
参考: docs/src/dev/commit-convention.md

格式: :emoji: type(scope): subject
发版: :bookmark: V<版本号>: <发版标题>
"""
import sys
import re

VALID_TYPES = {
    "feat", "fix", "docs", "style", "refactor",
    "perf", "test", "chore", "ci", "build",
}

# 发版: :bookmark: V1.2.3: 标题
RELEASE_PATTERN = re.compile(
    r"^:bookmark:\s+V\d+\.\d+\.\d+:\s+.+"
)

# 常规: :emoji: type(scope): subject  (scope 必填)
PATTERN = re.compile(
    r"^:[a-z_]+:\s+"
    r"(?P<type>" + "|".join(VALID_TYPES) + r")"
    r"\([a-zA-Z0-9_./*\-]+\)"
    r"!?:\s+.+"
)

SKIP_PATTERNS = [
    re.compile(r"^Merge (branch|pull request|remote-tracking)"),
    re.compile(r'^Revert "'),
]

GUIDANCE = """
╔══════════════════════════════════════════════════════════════╗
║              Commit message 格式不正确                        ║
╚══════════════════════════════════════════════════════════════╝

期望格式:  :emoji: type(scope): subject

  :emoji:  必须，GitHub 风格 shortcode
  type     必须: feat fix docs style refactor perf test chore ci build
  scope    必须: (frontend) (parser) (lexer) (typecheck) (middle)
                 (codegen) (vm) (runtime) (std) (util) (diagnostic)
                 (docs) (build) (ci) (test) (chore) (release) (meta) 等
  subject  中文，不超过 50 字符

发版格式:  :bookmark: V<版本号>: <发版标题>

示例:
  :sparkles: feat(parser): 添加闭包语法解析支持
  :bug: fix(vm): 修复栈帧溢出问题
  :memo: docs: 更新 README 安装说明
  :recycle: refactor(codesrc): 重构 IR 生成逻辑
  :wrench: chore(build): 更新 Cargo 依赖
  :bookmark: V0.8.0: 新增任务统计和数据分析功能

破坏性变更在 type 后加 !，并在 body 中写 BREAKING CHANGE:

详见: docs/src/dev/commit-convention.md
"""


def main():
    if len(sys.argv) < 2:
        print("Usage: check-commit-msg.py <commit-msg-file>", file=sys.stderr)
        sys.exit(1)

    msg_file = sys.argv[1]
    try:
        with open(msg_file, encoding="utf-8") as f:
            first_line = f.readline().strip()
    except FileNotFoundError:
        print(f"File not found: {msg_file}", file=sys.stderr)
        sys.exit(1)

    if not first_line:
        return

    for pat in SKIP_PATTERNS:
        if pat.match(first_line):
            return

    if PATTERN.match(first_line) or RELEASE_PATTERN.match(first_line):
        return

    print(GUIDANCE, file=sys.stderr)
    print(f'当前提交消息: "{first_line}"', file=sys.stderr)
    sys.exit(1)


if __name__ == "__main__":
    main()
