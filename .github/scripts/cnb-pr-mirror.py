#!/usr/bin/env python3
"""把 CNB 备份仓库上的合并请求镜像到 GitHub，便于在 GitHub 上核验与合并。

- CNB 上 open 的 PR（多数由 NPC 发起）以 cnb/pr-<编号> 分支推到本仓库，并开一个 GitHub PR；
- 源分支追加提交后，镜像分支与 PR 描述随之更新；
- GitHub PR 合并后，下一次运行关闭 CNB 侧同编号的 PR 并留下合并链接。

只处理源分支在同一仓库内的 PR；fork PR 需要人工处理。
环境变量：CNB_REPO、CNB_TOKEN、GITHUB_REPOSITORY，以及 gh 读取的 GH_TOKEN / SYNC_PAT_PRESENT。
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
import urllib.request

CNB_API = "https://api.cnb.cool"
CNB_WEB = "https://cnb.cool"
CNB_REPO = os.environ["CNB_REPO"]
CNB_TOKEN = os.environ["CNB_TOKEN"]
GH_REPO = os.environ["GITHUB_REPOSITORY"]
SYNC_PAT_PRESENT = os.environ.get("SYNC_PAT_PRESENT") == "true"

MARKER_TEMPLATE = "<!-- cnb-pr-mirror sha={sha} -->"
MARKER_RE = re.compile(r"<!-- cnb-pr-mirror sha=([0-9a-f]{7,40}) -->")


def redact(text: str) -> str:
    return text.replace(CNB_TOKEN, "***") if CNB_TOKEN else text


def run(*cmd: str, check: bool = True, input_text: str | None = None) -> str:
    """执行外部命令；失败信息里屏蔽令牌，避免进日志。"""
    proc = subprocess.run(cmd, capture_output=True, text=True, input=input_text)
    if check and proc.returncode != 0:
        raise RuntimeError(
            f"命令失败（{proc.returncode}）：{' '.join(cmd)}\n{proc.stdout}\n{proc.stderr}"
        )
    return proc.stdout


def cnb_api(method: str, path: str, payload: dict | None = None):
    data = json.dumps(payload).encode() if payload is not None else None
    request = urllib.request.Request(
        f"{CNB_API}/{path}",
        data=data,
        method=method,
        headers={
            "Authorization": f"Bearer {CNB_TOKEN}",
            "accept": "application/json",
            "content-type": "application/json",
        },
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        body = response.read().decode()
    return json.loads(body) if body.strip() else None


def gh_json(*args: str):
    return json.loads(run("gh", *args) or "null")


def short_ref(ref: str) -> str:
    return ref.removeprefix("refs/heads/")


def list_open_prs() -> list[dict]:
    prs: list[dict] = []
    page = 1
    while True:
        batch = cnb_api(
            "GET", f"{CNB_REPO}/-/pulls?state=open&page={page}&page_size=100"
        ) or []
        prs.extend(batch)
        if len(batch) < 100:
            return prs
        page += 1


def existing_pr(branch: str) -> dict | None:
    items = gh_json(
        "pr", "list", "--repo", GH_REPO, "--head", branch, "--state", "all",
        "--json", "number,state,url,title,body", "--limit", "1",
    ) or []
    return items[0] if items else None


def marker_sha(body: str) -> str | None:
    match = MARKER_RE.search(body or "")
    return match.group(1) if match else None


def build_body(pr: dict, sha: str, mirror_branch: str) -> str:
    author = pr.get("author") or {}
    who = f"@{author.get('username', '?')}"
    if author.get("nickname"):
        who += f"（{author['nickname']}）"
    if author.get("is_npc"):
        who += " · NPC"
    cnb_url = f"{CNB_WEB}/{CNB_REPO}/-/pulls/{pr['number']}"
    return "\n".join([
        "## 来自 CNB 的合并请求",
        "",
        "| 项 | 值 |",
        "| --- | --- |",
        f"| CNB PR | {cnb_url} |",
        f"| 发起人 | {who} |",
        f"| CNB 源分支 | `{short_ref(pr['head']['ref'])}` |",
        f"| 镜像分支 | `{mirror_branch}` |",
        f"| 提交 | `{sha}` |",
        "",
        "本 PR 由 GitHub Actions 从 CNB 备份仓库镜像而来：评审、CI 与合并都在 GitHub 进行。",
        "合并后 CNB 侧对应 PR 会被自动关闭，dev 由 CNB 的定时任务回灌。",
        "",
        MARKER_TEMPLATE.format(sha=sha),
    ])


def close_cnb_pr(number: str, gh_url: str) -> None:
    cnb_api(
        "POST",
        f"{CNB_REPO}/-/pulls/{number}/comments",
        {
            "body": (
                f"已在 GitHub 合并：{gh_url}\n\n"
                "本 PR 由 GitHub Actions 从 GitHub 侧自动关闭；CNB 的 dev 会在下一轮定时同步中更新。"
            )
        },
    )
    cnb_api("PATCH", f"{CNB_REPO}/-/pulls/{number}", {"state": "closed"})


def dispatch_ci(branch: str) -> str | None:
    """GITHUB_TOKEN 推送拉不起工作流，显式派发一次 CI，返回运行链接。"""
    proc = subprocess.run(
        ["gh", "workflow", "run", "ci.yml", "--repo", GH_REPO, "--ref", branch],
        capture_output=True, text=True,
    )
    if proc.returncode != 0:
        print(f"  派发 CI 失败：{proc.stderr.strip()}", file=sys.stderr)
        return None
    for _ in range(6):
        time.sleep(5)
        runs = gh_json(
            "run", "list", "--repo", GH_REPO, "--workflow", "ci.yml",
            "--branch", branch, "--limit", "1", "--json", "url",
        ) or []
        if runs:
            return runs[0]["url"]
    return None


def sync_pr(pr: dict) -> str:
    number = str(pr["number"])
    base = short_ref(pr["base"]["ref"])
    head_ref = short_ref(pr["head"]["ref"])
    head_repo = (pr.get("head") or {}).get("repo") or {}
    sha = pr["head"]["sha"]
    mirror_branch = f"cnb/pr-{number}"

    if head_repo.get("path") != CNB_REPO:
        return f"跳过：源分支在 {head_repo.get('path')}（fork PR 不镜像）"
    if head_ref.startswith("cnb/"):
        return "跳过：本身是镜像分支"

    existing = existing_pr(mirror_branch)
    if existing:
        if existing["state"] == "MERGED":
            close_cnb_pr(number, existing["url"])
            return "GitHub PR 已合并：关闭 CNB PR"
        if existing["state"] == "CLOSED":
            return "跳过：GitHub PR 已关闭（未合并）"
        if marker_sha(existing.get("body") or "") == sha:
            return "跳过：无新提交"

    if not run("git", "ls-remote", "--heads", "origin", base).strip():
        return f"跳过：GitHub 上没有目标分支 {base}"

    fetch_url = f"https://cnb:{CNB_TOKEN}@cnb.cool/{CNB_REPO}.git"
    run(
        "git", "fetch", "--no-tags", "--force", fetch_url,
        f"+refs/heads/{head_ref}:refs/remotes/cnb/{head_ref}",
    )
    fetched = run("git", "rev-parse", f"refs/remotes/cnb/{head_ref}").strip()
    run(
        "git", "push", "--force", "origin",
        f"refs/remotes/cnb/{head_ref}:refs/heads/{mirror_branch}",
    )

    title = f"[CNB #{number}] {pr['title']}".strip()
    body = build_body(pr, fetched, mirror_branch)

    if existing:
        if existing.get("title") != title or marker_sha(existing.get("body") or "") != fetched:
            proc = subprocess.run(
                ["gh", "pr", "edit", str(existing["number"]), "--repo", GH_REPO,
                 "--title", title, "--body-file", "-"],
                capture_output=True, text=True, input=body,
            )
            if proc.returncode != 0:
                raise RuntimeError(f"更新 PR 失败：{proc.stderr}")
        created = False
        gh_url = existing["url"]
    else:
        gh_url = run(
            "gh", "pr", "create", "--repo", GH_REPO, "--base", base,
            "--head", mirror_branch, "--title", title, "--body-file", "-",
            input_text=body,
        ).strip()
        created = True
        cnb_api(
            "POST",
            f"{CNB_REPO}/-/pulls/{number}/comments",
            {"body": f"已镜像到 GitHub，请在 GitHub 上评审与合并：{gh_url}"},
        )

    if not SYNC_PAT_PRESENT:
        run_url = dispatch_ci(mirror_branch)
        if run_url:
            subprocess.run(
                ["gh", "pr", "comment", "--repo", GH_REPO, str(existing["number"]) if existing else gh_url,
                 "--body", f"提交来自 CNB，已用 GITHUB_TOKEN 推送（不会自动触发工作流），这里显式派发了一次 CI：{run_url}"],
                capture_output=True, text=True,
            )

    return "已创建 GitHub PR" if created else "已更新 GitHub PR"


def main() -> int:
    prs = list_open_prs()
    if not prs:
        print("CNB 上没有 open 的合并请求。")
        return 0
    failures = 0
    for pr in prs:
        try:
            print(f"CNB PR #{pr['number']}: {sync_pr(pr)}")
        except Exception as exc:  # noqa: BLE001 - 单条 PR 失败不影响其余 PR
            failures += 1
            print(f"CNB PR #{pr['number']}: 失败 {redact(str(exc))}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
