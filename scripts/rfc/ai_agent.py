#!/usr/bin/env python3
"""
ai_agent.py — Transitional AI agent for RFC workflow.

This script reads a GitHub event, determines which task to perform, calls
an LLM (OpenAI-compatible API) to analyze the event, and posts a comment
or summary with the result.

Tasks:
  - issue_rfc_link:    Associate a newly-opened Issue with relevant RFCs
  - pr_consistency:    Check whether a PR implementation matches its RFC
  - progress_report:   Generate a project progress report (workflow_dispatch)
"""

import json
import os
import re
import sys

import httpx


# ── Constants ────────────────────────────────────────────────────────────────

RFC_ROOT = os.path.join("docs", "src", "design", "rfc")
AI_BASE_URL = os.environ.get("AI_BASE_URL", "https://api.openai.com/v1/chat/completions")
MODEL = os.environ.get("AI_MODEL", "gpt-4o-mini")
TEMPERATURE = 0.1
TIMEOUT = 30


# ── Task Detection ──────────────────────────────────────────────────────────

def determine_task_type(event_path):
    """Read the GitHub event JSON and determine which task to run.

    Args:
        event_path: Path to the GitHub event JSON file.

    Returns:
        One of 'issue_rfc_link', 'pr_consistency', 'progress_report',
        or None if the event is not actionable.
    """
    with open(event_path, 'r', encoding='utf-8') as f:
        event = json.load(f)

    action = event.get('action', '')

    if action == 'opened':
        if 'issue' in event:
            return 'issue_rfc_link'
        if 'pull_request' in event:
            pr_body = event['pull_request'].get('body', '') or ''
            if parse_rfc_id(pr_body):
                return 'pr_consistency'
        return None

    if action == 'workflow_dispatch':
        return 'progress_report'

    return None


# ── RFC ID Extraction ───────────────────────────────────────────────────────

def parse_rfc_id(text):
    """Extract the first RFC-XXX identifier from text.

    Args:
        text: String that may contain an RFC reference (e.g. 'RFC-042').

    Returns:
        The RFC ID string (e.g. 'RFC-042') or None if not found.
    """
    match = re.search(r'RFC-(\d{3,})', text, re.IGNORECASE)
    if match:
        return f"RFC-{match.group(1)}"
    return None


# ── Prompt Construction ──────────────────────────────────────────────────────

def build_issue_prompt(issue_body, rfc_index):
    """Build the prompt for Issue↔RFC association.

    Args:
        issue_body: The issue description text.
        rfc_index: List of RFC summary strings (e.g. ['RFC-001: Title']).

    Returns:
        A prompt string for the LLM.
    """
    rfc_list_str = "\n".join(f"- {r}" for r in rfc_index) if rfc_index else "(No RFCs yet)"
    return f"""You are an RFC triage assistant for the YaoXiang programming language project.

## Issue Body
{issue_body}

## Existing RFCs
{rfc_list_str}

## Task
Determine whether this issue describes a language design change that should
have an RFC, and whether it relates to any existing RFC.

### Classification Rules
- **Language design change**: Proposals for new syntax, new language features,
  semantic changes, type system changes, or breaking changes. These MUST have an RFC.
- **Not a language change**: Bug reports, documentation fixes, build system
  improvements, tooling enhancements, performance optimizations that don't
  change semantics, or general questions. These do NOT need a new RFC, but
  may still relate to an existing one.

## Output Format
Return ONLY valid JSON with NO additional text:
{{
    "is_language_change": true,
    "related_rfc": "RFC-XXX or null",
    "suggestion": "string — brief explanation and suggested action"
}}
"""


def build_pr_prompt(pr_title, pr_body, rfc_id):
    """Build the prompt for PR↔RFC consistency check.

    Args:
        pr_title: The PR title.
        pr_body: The PR description body.
        rfc_id: The RFC ID referenced in the PR (e.g. 'RFC-001').

    Returns:
        A prompt string for the LLM.
    """
    return f"""You are an RFC compliance reviewer for the YaoXiang programming language project.

## Pull Request
**Title**: {pr_title}

**Body**:
{pr_body}

**Referenced RFC**: {rfc_id}

## Task
Check whether this PR's implementation is consistent with the referenced RFC.

Evaluation criteria:
1. Does the PR's scope match the RFC's scope?
2. Are there any deviations, omissions, or contradictions?
3. Does the PR include necessary tests and documentation?

## Output Format
Return ONLY valid JSON with NO additional text:
{{
    "consistent": true,
    "issues_found": ["string — list of specific issues, empty if none"],
    "suggestion": "string — brief summary and recommended actions"
}}
"""


# ── LLM API Call ────────────────────────────────────────────────────────────

def call_llm(prompt, api_key):
    """Call the OpenAI-compatible API with the given prompt.

    Args:
        prompt: The full prompt text.
        api_key: OpenAI API key.

    Returns:
        The response content text from the LLM.

    Raises:
        httpx.HTTPStatusError: If the API returns a non-2xx status.
    """
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }

    payload = {
        "model": MODEL,
        "messages": [
            {"role": "user", "content": prompt},
        ],
        "temperature": TEMPERATURE,
    }

    with httpx.Client(timeout=TIMEOUT) as client:
        response = client.post(
            AI_BASE_URL,
            headers=headers,
            json=payload,
        )
        response.raise_for_status()
        data = response.json()
        return data["choices"][0]["message"]["content"]


# ── Response Parsing ─────────────────────────────────────────────────────────

def parse_llm_response(response_text, task_type):
    """Parse the LLM response text into a structured dict.

    Handles plain JSON and JSON inside markdown code fences.

    Args:
        response_text: Raw text from the LLM.
        task_type: One of 'issue_rfc_link', 'pr_consistency', 'progress_report'.

    Returns:
        A dict parsed from the JSON.

    Raises:
        ValueError: If the response cannot be parsed as valid JSON.
    """
    text = response_text.strip()

    # Strip markdown code fences if present
    if text.startswith("```"):
        # Remove leading ```json or ```
        text = re.sub(r'^```\w*\n?', '', text)
        # Remove trailing ```
        text = re.sub(r'\n?```\s*$', '', text)
        text = text.strip()

    try:
        return json.loads(text)
    except json.JSONDecodeError as e:
        raise ValueError(
            f"LLM response is not valid JSON for task '{task_type}': {e}\n"
            f"Raw response: {response_text[:500]}"
        )


# ── GitHub Comment Posting ───────────────────────────────────────────────────

def post_comment(issue_or_pr_number, body):
    """Post a comment to a GitHub Issue or PR.

    Uses GITHUB_REPOSITORY and GITHUB_TOKEN environment variables.

    Args:
        issue_or_pr_number: The issue or PR number.
        body: The comment body text.
    """
    repo = os.environ.get("GITHUB_REPOSITORY")
    token = os.environ.get("GITHUB_TOKEN", "")
    if not repo:
        print(f"  [SKIP] GITHUB_REPOSITORY not set — cannot post comment to #{issue_or_pr_number}")
        return

    url = f"https://api.github.com/repos/{repo}/issues/{issue_or_pr_number}/comments"
    headers = {
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github.v3+json",
    }
    payload = {"body": body}

    with httpx.Client(timeout=15) as client:
        resp = client.post(url, headers=headers, json=payload)
        resp.raise_for_status()


# ── Main Orchestration ──────────────────────────────────────────────────────

def main():
    """Orchestrate the AI agent workflow.

    1. Read environment variables (GITHUB_EVENT_PATH, OPENAI_API_KEY).
    2. Determine which task to run from the GitHub event payload.
    3. Execute the task using the LLM.
    4. Post the result as a comment (or to GITHUB_STEP_SUMMARY for workflow_dispatch).
    """
    event_path = os.environ.get("GITHUB_EVENT_PATH", "")
    api_key = os.environ.get("AI_API_KEY", "")

    if not event_path:
        print("Error: GITHUB_EVENT_PATH environment variable not set.")
        sys.exit(1)

    if not api_key:
        print("Error: AI_API_KEY environment variable not set.")
        sys.exit(1)

    task_type = determine_task_type(event_path)
    if task_type is None:
        print("No actionable event — skipping.")
        return

    print(f"Task: {task_type}")

    # Read the event to extract relevant data
    with open(event_path, 'r', encoding='utf-8') as f:
        event = json.load(f)

    result_body = ""

    if task_type == 'issue_rfc_link':
        issue_number = event['issue']['number']
        issue_body = event['issue'].get('body', '') or ''
        # Build a simple RFC index by scanning the RFC directories
        rfc_index = _build_rfc_index()
        prompt = build_issue_prompt(issue_body, rfc_index)
        raw = call_llm(prompt, api_key)
        parsed = parse_llm_response(raw, task_type)
        result_body = (
            f"## 🤖 RFC AI Agent — Issue Analysis\n\n"
            f"**Is language change**: {'Yes' if parsed.get('is_language_change') else 'No'}\n\n"
            f"**Related RFC**: {parsed.get('related_rfc', 'None')}\n\n"
            f"**Suggestion**: {parsed.get('suggestion', '')}\n"
        )
        post_comment(issue_number, result_body)
        print(result_body)

    elif task_type == 'pr_consistency':
        pr_info = event['pull_request']
        pr_number = pr_info['number']
        pr_title = pr_info.get('title', '')
        pr_body = pr_info.get('body', '') or ''
        rfc_id = parse_rfc_id(pr_body)
        prompt = build_pr_prompt(pr_title, pr_body, rfc_id)
        raw = call_llm(prompt, api_key)
        parsed = parse_llm_response(raw, task_type)
        issues = parsed.get('issues_found', [])
        result_body = (
            f"## 🤖 RFC AI Agent — PR Consistency Check\n\n"
            f"**Referenced RFC**: {rfc_id}\n\n"
            f"**Consistent**: {'Yes' if parsed.get('consistent') else 'No'}\n\n"
        )
        if issues:
            result_body += "**Issues found**:\n" + "\n".join(f"- {i}" for i in issues) + "\n\n"
        else:
            result_body += "**Issues found**: None\n\n"
        result_body += f"**Suggestion**: {parsed.get('suggestion', '')}\n"
        post_comment(pr_number, result_body)
        print(result_body)

    elif task_type == 'progress_report':
        # Output to GITHUB_STEP_SUMMARY instead of posting a comment
        rfc_index = _build_rfc_index()
        prompt = build_issue_prompt("Generate a progress report for the RFC workflow.", rfc_index)
        raw = call_llm(prompt, api_key)
        parsed = parse_llm_response(raw, task_type)
        summary = json.dumps(parsed, indent=2, ensure_ascii=False)

        step_summary = os.environ.get("GITHUB_STEP_SUMMARY", "")
        if step_summary:
            with open(step_summary, 'w', encoding='utf-8') as f:
                f.write(f"# RFC Progress Report\n\n```json\n{summary}\n```\n")
        print(f"Progress report:\n{summary}")


def _build_rfc_index():
    """Scan the RFC directory and build a human-readable index.

    Returns:
        A list of strings like ['RFC-001: Closures', 'RFC-002: Pattern Matching'].
    """
    rfc_root = os.path.join(os.environ.get("GITHUB_WORKSPACE", "."), RFC_ROOT)
    index = []

    if not os.path.isdir(rfc_root):
        return index

    for entry in sorted(os.listdir(rfc_root)):
        entry_path = os.path.join(rfc_root, entry)
        if not os.path.isdir(entry_path):
            continue

        # Look for the RFC file inside the directory (e.g. 001-closures/001-closures.md)
        for fname in os.listdir(entry_path):
            if fname.endswith('.md') and fname != 'index.md':
                filepath = os.path.join(entry_path, fname)
                try:
                    with open(filepath, 'r', encoding='utf-8') as f:
                        content = f.read()
                    # Extract title from frontmatter
                    title = _extract_title(content)
                    rfc_num = entry.split('-')[0] if '-' in entry else entry
                    index.append(f"RFC-{rfc_num}: {title}")
                except (OSError, UnicodeDecodeError):
                    pass
                break  # Only the first .md file per directory
        else:
            # No .md files in directory — just list the directory name
            index.append(f"RFC-{entry}: (no content)")

    return index


def _extract_title(content):
    """Extract title from frontmatter of an RFC markdown file.

    Args:
        content: Full text of the RFC markdown file.

    Returns:
        The title string, or a fallback if not found.
    """
    # Match YAML frontmatter between --- delimiters
    m = re.match(r'^---\s*\n(.*?)\n---', content, re.DOTALL)
    if m:
        frontmatter = m.group(1)
        title_m = re.search(r'^title:\s*(.+)$', frontmatter, re.MULTILINE)
        if title_m:
            return title_m.group(1).strip().strip('"\'')
    # Fallback: extract first heading
    heading_m = re.search(r'^#\s+(.+)$', content, re.MULTILINE)
    if heading_m:
        return heading_m.group(1).strip()
    return "(unknown)"


if __name__ == '__main__':
    main()
