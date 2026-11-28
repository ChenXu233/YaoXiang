#!/usr/bin/env bash
# CNB -> GitHub：把 CNB 的 Issue 按需镜像成 GitHub Issue。
#
# 由 CNB 的 issue.open 事件直接驱动，也由每日兜底与页面手动运行驱动。
# 镜像方向与 PR 一致：CNB 上 open 的 Issue 建到 GitHub，GH 上对应 Issue 关闭后回关 CNB。
#
# issue.open 是不可信事件，流水线直接 import 密钥仓库；密钥文件需声明
# allow_events 含 issue.open，否则 CNB 会拒绝注入 GH_USER / GH_TOKEN。
#
# 依赖：CNB_TOKEN（流水线临时令牌）、GH_USER / GH_TOKEN（密钥仓库 imports 注入）。
# 标签以 CNB 侧的标签表为权威源，镜像到 GitHub 时按原名透传；GitHub 上不存在的标签会被
# GitHub 自动创建，避免每边维护一份映射表（D3：不复制同义数据）。
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=mirror-common.sh
source "$SCRIPT_DIR/mirror-common.sh"

CNB_WEB="${CNB_WEB:-https://cnb.cool/$CNB_REPO}"

require_gh_creds || exit 1

# GitHub Issue 列表接口不按正文过滤，只能翻页取回全部（含 closed）后按镜像标记匹配。
# 标记写在 GitHub Issue 正文里，是判断「这条 GH Issue 对应哪条 CNB Issue」的唯一依据。
gh_issues="$(mktemp)"
trap 'rm -f "$gh_issues"' EXIT

page=1
while :; do
  # 拉取失败不能当作「没有已镜像 Issue」——否则会把已存在的 Issue 再建一遍。
  if ! batch="$(gh_call GET "/issues?state=all&per_page=100&page=$page")"; then
    echo "拉取 GitHub Issue 列表失败，停止镜像以避免重复创建" >&2
    exit 1
  fi
  count="$(printf '%s' "$batch" | jq 'length')"
  if [ -z "$count" ] || [ "$count" -eq 0 ]; then break; fi
  printf '%s' "$batch" | jq -c '.[]' >> "$gh_issues"
  if [ "$count" -lt 100 ]; then break; fi
  page=$((page + 1))
done

cnb_page=1
while :; do
  batch="$(cnb_call GET "/-/issues?state=open&page=$cnb_page&page_size=100")"
  count="$(printf '%s' "$batch" | jq 'length')"
  if [ -z "$count" ] || [ "$count" -eq 0 ]; then break; fi

  printf '%s' "$batch" | jq -c '.[]' | while IFS= read -r issue; do
    num="$(printf '%s' "$issue" | jq -r '.number')"
    # 私密 Issue 不镜像到公开 GitHub，避免把内部内容泄漏出去。
    if [ "$(printf '%s' "$issue" | jq -r '.invisible // false')" = "true" ]; then
      echo "Issue #$num：跳过（私密 Issue 不镜像）"
      continue
    fi
    title="$(printf '%s' "$issue" | jq -r '.title // ""')"
    body="$(printf '%s' "$issue" | jq -r '.body // ""')"
    author="$(printf '%s' "$issue" | jq -r '.author.username // ""')"
    nick="$(printf '%s' "$issue" | jq -r '.author.nickname // ""')"
    labels="$(printf '%s' "$issue" | jq -c '[.labels[]? | .name]')"

    marker="cnb-issue-mirror cnb/$CNB_REPO#$num"
    cnb_url="$CNB_WEB/-/issues/$num"

    # 用标记在已镜像的 GH Issue 中找对应项；没有就是「按需创建」的时机。
    match="$(jq -s --arg m "$marker" '[.[] | select((.body // "") | contains($m))] | .[0] // empty' "$gh_issues")"
    if [ -n "$match" ]; then
      gh_num="$(printf '%s' "$match" | jq -r '.number')"
      gh_state="$(printf '%s' "$match" | jq -r '.state')"
      if [ "$gh_state" = "closed" ]; then
        cnb_call POST "/-/issues/$num/comments" \
          "$(jq -n --arg b "已在 GitHub 关闭对应 Issue：$(printf '%s' "$match" | jq -r '.html_url')" '{body:$b}')" > /dev/null
        cnb_call PATCH "/-/issues/$num" '{"state":"closed"}' > /dev/null
        echo "Issue #$num：GitHub Issue #$gh_num 已关闭，已回关 CNB Issue"
      else
        echo "Issue #$num：已镜像到 GitHub Issue #$gh_num，无需重复创建"
      fi
      continue
    fi

    who="@$author"
    if [ -n "$nick" ]; then who="$who（$nick）"; fi
    gh_body="$(printf '%s\n' \
      "$body" \
      '' \
      '---' \
      "镜像来源：$cnb_url · 发起人：$who" \
      "<!-- $marker -->")"
    payload="$(jq -n --arg t "$title" --arg b "$gh_body" --argjson l "$labels" \
      '{title:$t, body:$b, labels:$l}')"

    created="$(gh_call POST "/issues" "$payload")" || {
      echo "Issue #$num：创建 GitHub Issue 失败" >&2
      continue
    }
    gh_url="$(printf '%s' "$created" | jq -r '.html_url // empty')"
    if [ -z "$gh_url" ]; then
      echo "Issue #$num：创建 GitHub Issue 失败：$(printf '%s' "$created" | jq -r '.message // .errors // "未知错误" | tostring')" >&2
      continue
    fi
    cnb_call POST "/-/issues/$num/comments" \
      "$(jq -n --arg b "已镜像到 GitHub，请在 GitHub 跟踪：$gh_url" '{body:$b}')" > /dev/null
    echo "Issue #$num：已创建 GitHub Issue $gh_url"
  done

  if [ "$count" -lt 100 ]; then break; fi
  cnb_page=$((cnb_page + 1))
done

echo "CNB Issue 镜像完成。"
