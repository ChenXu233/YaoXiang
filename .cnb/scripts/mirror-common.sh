#!/usr/bin/env bash
# CNB <-> GitHub 镜像脚本共用的 API 调用函数。
# 由 mirror-prs.sh / mirror-issues.sh source；凭据从环境变量读取，不拼进 URL，
# 避免 git / curl 的报错把令牌写进构建日志。
#
# 环境变量均可覆盖，便于在本地用假 API 做幂等性验证：
#   CNB_REPO / GH_OWNER / CNB_API / GH_API / CNB_TOKEN / GH_USER / GH_TOKEN
CNB_REPO="${CNB_REPO:-ChenXu233/YaoXiang}"
GH_OWNER="${GH_OWNER:-ChenXu233}"
CNB_API="${CNB_API:-https://api.cnb.cool/$CNB_REPO}"
GH_API="${GH_API:-https://api.github.com/repos/$CNB_REPO}"

require_gh_creds() {
  if [ -z "${GH_USER:-}" ] || [ -z "${GH_TOKEN:-}" ]; then
    echo "缺少 GH_USER / GH_TOKEN：检查密钥仓库 ChenXu233/secrets 的 yaoxiang-pr-sync.yml" >&2
    return 1
  fi
}

cnb_call() {
  if [ $# -ge 3 ] && [ -n "$3" ]; then
    curl -sS -f -X "$1" -H "Authorization: Bearer $CNB_TOKEN" \
      -H "accept: application/json" -H "content-type: application/json" \
      --data "$3" "$CNB_API$2"
  else
    curl -sS -f -X "$1" -H "Authorization: Bearer $CNB_TOKEN" \
      -H "accept: application/json" "$CNB_API$2"
  fi
}

gh_call() {
  if [ $# -ge 3 ] && [ -n "$3" ]; then
    curl -sS -f -X "$1" -H "Authorization: Bearer $GH_TOKEN" \
      -H "accept: application/vnd.github+json" -H "X-GitHub-Api-Version: 2022-11-28" \
      -H "content-type: application/json" --data "$3" "$GH_API$2"
  else
    curl -sS -f -X "$1" -H "Authorization: Bearer $GH_TOKEN" \
      -H "accept: application/vnd.github+json" -H "X-GitHub-Api-Version: 2022-11-28" "$GH_API$2"
  fi
}
