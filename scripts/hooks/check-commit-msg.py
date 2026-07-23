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

# 合法的 emoji 代码（来自 docs/src/dev/commit-convention.md）
VALID_EMOJIS = {
    "art", "zap", "racehorse", "fire", "bug", "ambulance",
    "sparkles", "memo", "rocket", "lipstick", "tada",
    "white_check_mark", "lock", "apple", "penguin",
    "checkered_flag", "robot", "green_apple", "bookmark",
    "rotating_light", "construction", "green_heart",
    "arrow_down", "arrow_up", "pushpin", "construction_worker",
    "chart_with_upwards_trend", "recycle", "hammer",
    "heavy_minus_sign", "whale", "heavy_plus_sign", "wrench",
    "globe_with_meridians", "pencil2", "hankey", "rewind",
    "twisted_rightwards_arrows", "package", "alien", "truck",
    "page_facing_up", "boom", "bento", "ok_hand", "wheelchair",
    "bulb", "beers", "speech_balloon", "card_file_box",
    "loud_sound", "mute", "busts_in_silhouette",
    "children_crossing", "building_construction", "iphone",
    "clown_face", "egg", "see_no_evil", "camera_flash",
}

# 合法的 scope（来自 docs/src/dev/commit-convention.md）
VALID_SCOPES = {
    # 顶层模块
    "frontend", "middle", "backends", "std", "formatter",
    "lsp", "package", "util",
    # 前端子模块
    "parser", "lexer", "typecheck", "types",
    # 中间层子模块
    "codegen", "monomorphize", "lifetime",
    # 后端子模块
    "repl", "shell", "runtime",
    # 文档作用域
    "docs", "design", "plan", "rfc",
    # 其他作用域
    "build", "ci", "test", "release", "meta",
}

EMOJI_PATTERN = "|".join(VALID_EMOJIS)
SCOPE_PATTERN = "|".join(VALID_SCOPES)

# 发版: :bookmark: V1.2.3: 标题
RELEASE_PATTERN = re.compile(
    r"^:bookmark:\s+V\d+\.\d+\.\d+:\s+.+"
)

# 常规: :emoji: type(scope): subject  (scope 必填)
PATTERN = re.compile(
    r"^:(?P<emoji>" + EMOJI_PATTERN + r"):\s+"
    r"(?P<type>" + "|".join(VALID_TYPES) + r")"
    r"\((?P<scope>" + SCOPE_PATTERN + r")\)"
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

  :emoji:  必须，GitHub 风格 shortcode，必须是合法的 gitmoji 代码
  type     必须: feat fix docs style refactor perf test chore ci build
  scope    必须（基于 src/ 目录结构）:
           frontend parser lexer typecheck types middle codegen
           monomorphize lifetime backends repl shell runtime
           std formatter lsp package util docs design plan
           build ci test release meta
  subject  中文，不超过 50 字符

常用 emoji 示例:
  :sparkles: feat(parser): 添加闭包语法解析支持
  :bug: fix(repl): 修复多行输入时补全器失效
  :memo: docs(design): 更新所有权模型规范
  :recycle: refactor(typecheck): 分离原语值类型与 Dup 浅拷贝语义
  :wrench: chore(build): 更新 Cargo 依赖
  :bookmark: V0.7.2: REPL 重写与类型系统改进

完整 emoji 列表: docs/src/dev/commit-convention.md

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
