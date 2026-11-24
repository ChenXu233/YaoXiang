#!/usr/bin/env bash
# GitHub 主仓库 -> CNB 备份仓库：推送全部分支与标签。
#
# 只推送、不删除：CNB 上 GitHub 没有的引用（NPC 的工作分支、CNB 侧 PR 分支）必须保留。
# dev/main 若出现 GitHub 没有的提交（例如有人直接在 CNB 页面合并了 PR），本脚本拒绝覆盖并以非零退出码停下。
set -eu

SRC="https://github.com/ChenXu233/YaoXiang.git"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# 每次都从 GitHub 全量裸克隆：不依赖工作区的克隆深度，避免浅克隆推送失败。
git clone --bare --quiet "$SRC" "$WORK/gh.git"
cd "$WORK/gh.git"

# 用流水线临时令牌（CNB_TOKEN，随任务销毁）认证推送到本仓库；令牌不拼进 URL，避免进日志。
git config credential.helper '!f() { echo username=cnb; echo password="${CNB_TOKEN}"; }; f'

git fetch --quiet "$CNB_REPO_URL_HTTPS" \
  '+refs/heads/dev:refs/cnb/dev' '+refs/heads/main:refs/cnb/main' || true

for branch in dev main; do
  if git rev-parse --verify --quiet "refs/cnb/$branch" >/dev/null; then
    if ! git merge-base --is-ancestor "refs/cnb/$branch" "refs/heads/$branch"; then
      echo "CNB 的 $branch 含有 GitHub 上没有的提交，停止同步以避免覆盖："
      git log --oneline "refs/heads/$branch..refs/cnb/$branch" | head -n 20
      echo "处理方式：先把这些改动落到 GitHub（或在 GitHub 合并对应 PR），再等下一轮。"
      exit 1
    fi
  fi
done

git push --quiet --force "$CNB_REPO_URL_HTTPS" \
  'refs/heads/*:refs/heads/*' 'refs/tags/*:refs/tags/*'

echo "已同步 GitHub -> CNB：dev=$(git rev-parse --short refs/heads/dev) main=$(git rev-parse --short refs/heads/main)"
