#!/usr/bin/env bash
# CNB -> GitHub：把 CNB 上 open 的合并请求镜像成 GitHub 上的 cnb/pr-<编号> 分支与 PR。
#
# 由 CNB 的定时任务（可信事件，以负责人身份运行）驱动，不依赖 GitHub Actions。
# 评审、CI 与合并都在 GitHub 进行；GitHub PR 合并后，本脚本回关 CNB 的同号 PR。
#
# 需要环境变量：CNB_TOKEN（流水线临时令牌）、GH_USER / GH_TOKEN（密钥仓库 imports 注入）。
set -e

CNB_REPO="ChenXu233/YaoXiang"
GH_OWNER="ChenXu233"
CNB_API="https://api.cnb.cool/$CNB_REPO"
GH_API="https://api.github.com/repos/$CNB_REPO"
CNB_GIT="https://cnb:$CNB_TOKEN@cnb.cool/$CNB_REPO.git"
GH_GIT="https://$GH_USER:$GH_TOKEN@github.com/$CNB_REPO.git"

if [ -z "$GH_USER" ] || [ -z "$GH_TOKEN" ]; then
  echo "缺少 GH_USER / GH_TOKEN：检查密钥仓库 ChenXu233/secrets 的 yaoxiang-pr-sync.yml" >&2
  exit 1
fi

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

PRS="$(mktemp)"
trap 'rm -f "$PRS"' EXIT
: > "$PRS"

page=1
while :; do
  batch="$(cnb_call GET "/-/pulls?state=open&page=$page&page_size=100")"
  count="$(printf '%s' "$batch" | jq 'length')"
  if [ -z "$count" ] || [ "$count" -eq 0 ]; then
    break
  fi
  printf '%s' "$batch" | jq -r '.[] | [
      (.number | tostring),
      (.head.ref // "" | sub("^refs/heads/"; "")),
      (.head.repo.path // ""),
      (.base.ref // "" | sub("^refs/heads/"; "")),
      (.head.sha // ""),
      (.title // "" | gsub("[\t\n\r]"; " ")),
      (.author.username // ""),
      (.author.nickname // ""),
      ((.author.is_npc // false) | tostring)
    ] | @tsv' >> "$PRS"
  if [ "$count" -lt 100 ]; then
    break
  fi
  page=$((page + 1))
done

if [ ! -s "$PRS" ]; then
  echo "CNB 上没有 open 的合并请求。"
  exit 0
fi

TAB="$(printf '\t')"

while IFS="$TAB" read -r num head_ref head_repo base_ref sha title author nick is_npc; do
  mirror="cnb/pr-$num"

  if [ "$head_repo" != "$CNB_REPO" ]; then
    echo "PR #$num：跳过（源分支在 $head_repo，fork PR 不镜像）"
    continue
  fi

  case "$head_ref" in
    cnb/*)
      echo "PR #$num：跳过（源分支本身是镜像分支 $head_ref）"
      continue
      ;;
  esac

  existing="$(gh_call GET "/pulls?state=all&head=$GH_OWNER:$mirror&per_page=1")"
  gh_state="$(printf '%s' "$existing" | jq -r 'if length > 0 then .[0].state else "" end')"

  if [ "$gh_state" = "MERGED" ]; then
    gh_url="$(printf '%s' "$existing" | jq -r '.[0].html_url')"
    body="$(jq -n --arg b "已在 GitHub 合并：$gh_url" '{body: $b}')"
    cnb_call POST "/-/pulls/$num/comments" "$body" > /dev/null
    cnb_call PATCH "/-/pulls/$num" '{"state":"closed"}' > /dev/null
    echo "PR #$num：GitHub PR 已合并，已关闭 CNB PR"
    continue
  fi

  if [ "$gh_state" = "CLOSED" ]; then
    echo "PR #$num：跳过（GitHub PR 已关闭但未合并）"
    continue
  fi

  if [ "$gh_state" = "OPEN" ]; then
    old_body="$(printf '%s' "$existing" | jq -r '.[0].body // ""')"
    if printf '%s' "$old_body" | grep -q "cnb-pr-mirror sha=$sha"; then
      echo "PR #$num：无新提交"
      continue
    fi
  fi

  if ! git fetch --no-tags --force "$CNB_GIT" "+refs/heads/$head_ref:refs/remotes/cnb/$head_ref" > /dev/null 2>&1; then
    echo "PR #$num：跳过（源分支 $head_ref 已不存在）"
    continue
  fi
  fetched="$(git rev-parse "refs/remotes/cnb/$head_ref")"

  if ! git push --quiet --force "$GH_GIT" "refs/remotes/cnb/$head_ref:refs/heads/$mirror" > /dev/null 2>&1; then
    echo "PR #$num：推送到 GitHub 失败" >&2
    continue
  fi

  cnb_url="https://cnb.cool/$CNB_REPO/-/pulls/$num"
  who="@$author"
  if [ -n "$nick" ]; then who="$who（$nick）"; fi
  if [ "$is_npc" = "true" ]; then who="$who · NPC"; fi

  body_md="$(printf '%s\n' \
    '## 来自 CNB 的合并请求' \
    '' \
    '| 项 | 值 |' \
    '| --- | --- |' \
    "| CNB PR | $cnb_url |" \
    "| 发起人 | $who |" \
    "| CNB 源分支 | $head_ref |" \
    "| 镜像分支 | $mirror |" \
    "| 提交 | $fetched |" \
    '' \
    '本 PR 由 CNB 的定时流水线镜像而来：评审、CI 与合并都在 GitHub 进行。' \
    '合并后 CNB 侧同号 PR 会被自动关闭，dev 由 CNB 的定时任务回灌。' \
    '' \
    "<!-- cnb-pr-mirror sha=$fetched -->")"

  pr_title="[CNB #$num] $title"

  if [ "$gh_state" = "OPEN" ]; then
    pr_number="$(printf '%s' "$existing" | jq -r '.[0].number')"
    payload="$(jq -n --arg t "$pr_title" --arg b "$body_md" '{title: $t, body: $b}')"
    gh_call PATCH "/pulls/$pr_number" "$payload" > /dev/null
    echo "PR #$num：已更新 GitHub PR #$pr_number"
  else
    if ! git ls-remote --heads "$GH_GIT" "$base_ref" | grep -q .; then
      echo "PR #$num：跳过（GitHub 上没有目标分支 $base_ref）"
      continue
    fi
    payload="$(jq -n --arg t "$pr_title" --arg b "$body_md" --arg h "$mirror" --arg base "$base_ref" \
      '{title: $t, body: $b, head: $h, base: $base}')"
    created="$(gh_call POST "/pulls" "$payload")"
    gh_url="$(printf '%s' "$created" | jq -r '.html_url // empty')"
    if [ -z "$gh_url" ]; then
      echo "PR #$num：创建 GitHub PR 失败：$(printf '%s' "$created" | jq -r '.message // .errors // "未知错误" | tostring')" >&2
      continue
    fi
    comment="$(jq -n --arg b "已镜像到 GitHub，请在 GitHub 评审与合并：$gh_url" '{body: $b}')"
    cnb_call POST "/-/pulls/$num/comments" "$comment" > /dev/null
    echo "PR #$num：已创建 GitHub PR $gh_url"
  fi
done < "$PRS"

echo "CNB PR 镜像完成。"
