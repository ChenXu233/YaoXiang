#!/usr/bin/env python3
"""Unit tests for rfc_ai_agent.py — the transitional AI agent."""

import json
import os
import sys
import tempfile

import pytest

# Add scripts dir to path for import
_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, _SCRIPT_DIR)

import rfc_ai_agent as agent


# ══════════════════════════════════════════════════════════════════════════
# ── determine_task_type ──────────────────────────────────────────────────
# ══════════════════════════════════════════════════════════════════════════

def _write_event_file(content):
    """Helper: write event JSON to a temp file and return (path, fd)."""
    fd, path = tempfile.mkstemp(suffix='.json', text=True)
    with os.fdopen(fd, 'w') as f:
        json.dump(content, f)
    return path


def test_determine_task_type_issue():
    """issues.opened event should return 'issue_rfc_link'."""
    event = {
        'action': 'opened',
        'issue': {
            'number': 42,
            'body': 'This is a suggestion about adding closures.',
        },
    }
    path = _write_event_file(event)
    try:
        result = agent.determine_task_type(path)
        assert result == 'issue_rfc_link'
    finally:
        os.unlink(path)


def test_determine_task_type_pr():
    """PR opened with RFC-XXX in body should return 'pr_consistency'."""
    event = {
        'action': 'opened',
        'pull_request': {
            'number': 100,
            'title': 'Implement closures per RFC-001',
            'body': 'This PR implements RFC-001 about closures.',
        },
    }
    path = _write_event_file(event)
    try:
        result = agent.determine_task_type(path)
        assert result == 'pr_consistency'
    finally:
        os.unlink(path)


def test_determine_task_type_workflow_dispatch():
    """workflow_dispatch event should return 'progress_report'."""
    event = {
        'action': 'workflow_dispatch',
    }
    path = _write_event_file(event)
    try:
        result = agent.determine_task_type(path)
        assert result == 'progress_report'
    finally:
        os.unlink(path)


def test_determine_task_type_unmatched():
    """Unmatched events should return None."""
    event = {
        'action': 'labeled',
        'issue': {'number': 1, 'body': 'Just a label change.'},
    }
    path = _write_event_file(event)
    try:
        result = agent.determine_task_type(path)
        assert result is None
    finally:
        os.unlink(path)


def test_determine_task_type_pr_no_rfc():
    """PR opened without RFC-XXX in body should return None."""
    event = {
        'action': 'opened',
        'pull_request': {
            'number': 101,
            'title': 'Fix typo',
            'body': 'Minor documentation fix.',
        },
    }
    path = _write_event_file(event)
    try:
        result = agent.determine_task_type(path)
        assert result is None
    finally:
        os.unlink(path)


# ══════════════════════════════════════════════════════════════════════════
# ── build_issue_prompt ───────────────────────────────────────────────────
# ══════════════════════════════════════════════════════════════════════════

def test_build_issue_prompt():
    """build_issue_prompt should include issue body, rfc list, and JSON format."""
    issue_body = "I think YaoXiang should support algebraic data types."
    rfc_index = ["RFC-001: Closures", "RFC-002: Pattern Matching"]

    prompt = agent.build_issue_prompt(issue_body, rfc_index)

    assert issue_body in prompt
    assert "RFC-001: Closures" in prompt
    assert "RFC-002: Pattern Matching" in prompt
    assert "is_language_change" in prompt
    assert "related_rfc" in prompt
    assert "suggestion" in prompt
    assert '"is_language_change": true' in prompt or '"is_language_change": false' in prompt or 'bool' in prompt
    assert '"related_rfc": "RFC-XXX or null"' in prompt or '"related_rfc" : "RFC-XXX or null"' in prompt


# ══════════════════════════════════════════════════════════════════════════
# ── build_pr_prompt ──────────────────────────────────────────────────────
# ══════════════════════════════════════════════════════════════════════════

def test_build_pr_prompt():
    """build_pr_prompt should include PR info, referenced RFC, and JSON format."""
    pr_title = "feat: implement closures"
    pr_body = "This PR adds closure support as described in RFC-001."
    rfc_id = "RFC-001"

    prompt = agent.build_pr_prompt(pr_title, pr_body, rfc_id)

    assert pr_title in prompt
    assert pr_body in prompt
    assert rfc_id in prompt
    assert "consistent" in prompt
    assert "issues_found" in prompt
    assert "suggestion" in prompt


# ══════════════════════════════════════════════════════════════════════════
# ── parse_llm_response ───────────────────────────────────────────────────
# ══════════════════════════════════════════════════════════════════════════

def test_parse_llm_response_plain_json():
    """Parse plain JSON object from LLM response."""
    raw = '{"is_language_change": true, "related_rfc": "RFC-001", "suggestion": "Good idea."}'
    result = agent.parse_llm_response(raw, 'issue')
    assert result['is_language_change'] is True
    assert result['related_rfc'] == 'RFC-001'
    assert result['suggestion'] == 'Good idea.'


def test_parse_llm_response_markdown_json():
    """Parse JSON inside markdown code fence."""
    raw = '```json\n{"is_language_change": false, "related_rfc": null, "suggestion": "Not a language change."}\n```'
    result = agent.parse_llm_response(raw, 'issue')
    assert result['is_language_change'] is False
    assert result['related_rfc'] is None


def test_parse_llm_response_pr_consistency():
    """Parse PR consistency JSON from LLM response."""
    raw = '{"consistent": true, "issues_found": [], "suggestion": "All good."}'
    result = agent.parse_llm_response(raw, 'pr_consistency')
    assert result['consistent'] is True
    assert result['issues_found'] == []


def test_parse_llm_response_invalid_json():
    """Invalid JSON should raise ValueError with helpful message."""
    raw = 'This is not JSON at all.'
    with pytest.raises(ValueError, match='LLM response'):
        agent.parse_llm_response(raw, 'issue')


# ══════════════════════════════════════════════════════════════════════════
# ── call_llm (mock httpx) ────────────────────────────────────────────────
# ══════════════════════════════════════════════════════════════════════════

def test_call_llm_success(monkeypatch):
    """call_llm should return the response text from OpenAI-compatible API."""
    import httpx

    expected_text = '{"is_language_change": false, "related_rfc": null, "suggestion": "OK"}'

    class FakeResponse:
        status_code = 200

        def raise_for_status(self):
            pass

        def json(self):
            return {
                'choices': [{
                    'message': {'content': expected_text},
                }],
            }

    class FakeClient:
        def __init__(self, *args, **kwargs):
            pass

        def __enter__(self):
            return self

        def __exit__(self, *args):
            pass

        def post(self, url, **kwargs):
            assert 'api.openai.com' in url or 'openai' in url.lower()
            return FakeResponse()

    monkeypatch.setattr(httpx, 'Client', lambda *a, **kw: FakeClient())
    result = agent.call_llm('Test prompt', 'sk-test-key')
    assert result == expected_text


def test_call_llm_timeout(monkeypatch):
    """call_llm should set 30s timeout."""
    import httpx

    captured = {}

    class FakeResponse:
        status_code = 200

        def raise_for_status(self):
            pass

        def json(self):
            return {'choices': [{'message': {'content': '{}'}}]}

    class FakeClient:
        def __init__(self, *args, timeout=None, **kwargs):
            captured['timeout'] = timeout

        def __enter__(self):
            return self

        def __exit__(self, *args):
            pass

        def post(self, url, **kwargs):
            return FakeResponse()

    monkeypatch.setattr(httpx, 'Client', lambda *a, **kw: FakeClient(*a, **kw))
    agent.call_llm('Test prompt', 'sk-test-key')
    assert captured['timeout'] == 30, f"Expected timeout=30, got {captured.get('timeout')}"


# ══════════════════════════════════════════════════════════════════════════
# ── parse_rfc_id ─────────────────────────────────────────────────────────
# ══════════════════════════════════════════════════════════════════════════

def test_parse_rfc_id_from_body():
    """Parse RFC-XXX from PR body."""
    rfc_id = agent.parse_rfc_id("This implements RFC-042 about closures.")
    assert rfc_id == 'RFC-042'


def test_parse_rfc_id_no_match():
    """Return None when no RFC ID found."""
    rfc_id = agent.parse_rfc_id("Just a regular PR with no RFC reference.")
    assert rfc_id is None


# ══════════════════════════════════════════════════════════════════════════
# ── main (end-to-end via monkeypatch) ────────────────────────────────────
# ══════════════════════════════════════════════════════════════════════════

def test_main_issue_rfc_link(monkeypatch):
    """main() should run issue_rfc_link task with mocked LLM."""
    import httpx

    event = {
        'action': 'opened',
        'issue': {'number': 42, 'body': 'Add pattern matching.'},
    }
    path = _write_event_file(event)

    monkeypatch.setenv('GITHUB_EVENT_PATH', path)
    monkeypatch.setenv('OPENAI_API_KEY', 'sk-test-key')

    class FakeResponse:
        status_code = 200

        def raise_for_status(self):
            pass

        def json(self):
            return {
                'choices': [{
                    'message': {
                        'content': '{"is_language_change": true, "related_rfc": "RFC-002", "suggestion": "Relates to pattern matching RFC."}'
                    },
                }],
            }

    class FakeClient:
        def __init__(self, *args, **kwargs):
            pass

        def __enter__(self):
            return self

        def __exit__(self, *args):
            pass

        def post(self, url, **kwargs):
            return FakeResponse()

    monkeypatch.setattr(httpx, 'Client', lambda *a, **kw: FakeClient(*a, **kw))

    # Mock post_comment to avoid actual HTTP calls to GitHub
    posted = []

    def fake_post_comment(number, body):
        posted.append((number, body))

    monkeypatch.setattr(agent, 'post_comment', fake_post_comment)

    agent.main()

    assert len(posted) == 1
    number, body = posted[0]
    assert number == 42
    assert 'Is language change' in body
    assert 'RFC-002' in body


def test_main_unmatched_event(monkeypatch):
    """main() should handle unmatched events gracefully."""
    event = {
        'action': 'labeled',
        'issue': {'number': 1, 'body': 'Just a label.'},
    }
    path = _write_event_file(event)

    monkeypatch.setenv('GITHUB_EVENT_PATH', path)
    monkeypatch.setenv('OPENAI_API_KEY', 'sk-test-key')

    # Should not crash
    agent.main()
