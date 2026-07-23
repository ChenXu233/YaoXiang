#!/usr/bin/env python3
"""Unit tests for check_tracking.py frontmatter parser and validator."""

import os
import sys
import tempfile


import check_tracking as crt

# Add scripts dir to path for import
_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, _SCRIPT_DIR)


# ── parse_frontmatter ────────────────────────────────────────────────────

def test_parse_frontmatter_full():
    """Parse a full frontmatter with all fields including list fields."""
    content = """---
title: "RFC-999: Test RFC"
status: "草案"
author: "Tester"
created: "2026-07-01"
updated: "2026-07-03"
issue: "#123"
issues_impl:
  - "#456"
  - "#789"
pr_impl:
  - "#101"
---

# RFC-999: Test RFC
"""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.md', delete=False) as f:
        f.write(content)
        f.flush()
        path = f.name
    try:
        result = crt.parse_frontmatter(path)
        assert result['title'] == 'RFC-999: Test RFC'
        assert result['status'] == '草案'
        assert result['author'] == 'Tester'
        assert result['created'] == '2026-07-01'
        assert result['updated'] == '2026-07-03'
        assert result['issue'] == '#123'
        assert result['issues_impl'] == ['#456', '#789']
        assert result['pr_impl'] == ['#101']
    finally:
        os.unlink(path)


def test_parse_frontmatter_minimal():
    """Parse frontmatter without issue/impl fields."""
    content = """---
title: "RFC-001: Minimal"
author: "Tester"
created: "2025-01-01"
updated: "2025-06-01"
---

# RFC-001: Minimal
"""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.md', delete=False) as f:
        f.write(content)
        f.flush()
        path = f.name
    try:
        result = crt.parse_frontmatter(path)
        assert result['title'] == 'RFC-001: Minimal'
        assert result['author'] == 'Tester'
        assert 'issue' not in result
        assert 'issues_impl' not in result
    finally:
        os.unlink(path)


def test_parse_frontmatter_no_frontmatter():
    """File without frontmatter returns empty dict."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.md', delete=False) as f:
        f.write("# Just a heading\n\nNo frontmatter here.\n")
        f.flush()
        path = f.name
    try:
        result = crt.parse_frontmatter(path)
        assert result == {}
    finally:
        os.unlink(path)


# ── scan_directory ──────────────────────────────────────────────────────

def test_scan_directory_integration():
    """Integration test with temp directory and a real RFC file."""
    with tempfile.TemporaryDirectory() as tmpdir:
        draft_dir = os.path.join(tmpdir, "draft")
        os.makedirs(draft_dir)
        rfc_path = os.path.join(draft_dir, "999-test-rfc.md")
        with open(rfc_path, 'w', encoding='utf-8') as f:
            f.write("""---
title: "RFC-999: Integration Test"
status: "草案"
author: "Tester"
created: "2026-07-01"
updated: "2026-07-03"
issue: "#999"
---

# RFC-999
""")

        records = crt.scan_rfcs(tmpdir)
        assert len(records) == 1
        r = records[0]
        assert r['filename'] == '999-test-rfc.md'
        assert r['state'] == '草案'
        assert r['title'] == 'RFC-999: Integration Test'
        assert r['issue'] == '#999'


def test_scan_directory_skip_non_md():
    """Non-.md files should be skipped."""
    with tempfile.TemporaryDirectory() as tmpdir:
        draft_dir = os.path.join(tmpdir, "draft")
        os.makedirs(draft_dir)
        # Create a .md file
        with open(os.path.join(draft_dir, "valid.md"), 'w', encoding='utf-8') as f:
            f.write("""---
title: "Valid"
author: "Tester"
created: "2026-07-01"
updated: "2026-07-03"
---

# Valid
""")
        # Create non-.md files
        with open(os.path.join(draft_dir, "notes.txt"), 'w') as f:
            f.write("not an rfc")
        with open(os.path.join(draft_dir, "image.png"), 'w') as f:
            f.write("binary")

        records = crt.scan_rfcs(tmpdir)
        assert len(records) == 1
        assert records[0]['filename'] == 'valid.md'


def test_scan_directory_skip_special_files():
    """Special files (index.md, RFC_TEMPLATE.md, etc.) should be skipped."""
    with tempfile.TemporaryDirectory() as tmpdir:
        draft_dir = os.path.join(tmpdir, "draft")
        os.makedirs(draft_dir)
        # These should be skipped
        for name in ['index.md', 'RFC_TEMPLATE.md', 'EXAMPLE_full_feature_proposal.md', 'README.md']:
            with open(os.path.join(draft_dir, name), 'w', encoding='utf-8') as f:
                f.write("---\ntitle: skip\n---\n")
        # A real RFC
        with open(os.path.join(draft_dir, "001-real-rfc.md"), 'w', encoding='utf-8') as f:
            f.write("---\ntitle: Real\nauthor: Tester\ncreated: \"2026-07-01\"\nupdated: \"2026-07-03\"\n---\n")

        records = crt.scan_rfcs(tmpdir)
        assert len(records) == 1
        assert records[0]['filename'] == '001-real-rfc.md'


# ── validation (ERROR vs WARNING) ──────────────────────────────────────

def test_new_file_missing_issue_is_error():
    """Files after 2026-07-03 missing issue should trigger ERROR."""
    frontmatter = {
        'title': 'Test RFC',
        'author': 'Tester',
        'created': '2026-07-04',  # After cutoff
        'updated': '2026-07-04',
        # no 'issue' field
    }
    errors = crt.validate_rfc(frontmatter, 'draft/test.md', 'draft')
    assert any(e[0] == 'ERROR' and 'issue' in e[1] for e in errors)


def test_old_file_missing_issue_is_warning():
    """Files before 2026-07-03 missing issue should only WARN."""
    frontmatter = {
        'title': 'Old RFC',
        'author': 'Tester',
        'created': '2025-06-01',
        'updated': '2025-06-01',
        # no 'issue' field
    }
    errors = crt.validate_rfc(frontmatter, 'draft/old.md', 'draft')
    assert any(e[0] == 'WARNING' and 'issue' in e[1] for e in errors)
    assert not any(e[0] == 'ERROR' for e in errors)


def test_new_file_with_issue_passes():
    """Files after cutoff WITH issue field should pass."""
    frontmatter = {
        'title': 'New RFC',
        'author': 'Tester',
        'created': '2026-07-04',
        'updated': '2026-07-04',
        'issue': '#100',
    }
    errors = crt.validate_rfc(frontmatter, 'draft/new.md', 'draft')
    issue_errors = [e for e in errors if 'issue' in e[1]]
    assert len(issue_errors) == 0


# ── derive_state ─────────────────────────────────────────────────────────

def test_derive_state_from_directory():
    """State should be derived from directory name."""
    assert crt.derive_state('draft') == '草案'
    assert crt.derive_state('review') == '审核中'
    assert crt.derive_state('accepted') == '已接受'

    assert crt.derive_state('deprecated') == '已废弃'
    assert crt.derive_state('rejected') == '已拒绝'
    assert crt.derive_state('unknown') == '未知'


# ── generate_tracking_md ─────────────────────────────────────────────────

def test_generate_tracking_md_creates_table():
    """TRACKING.md should contain a markdown table."""
    records = [
        {
            'filename': '001-test.md',
            'title': 'Test RFC',
            'state': '草案',
            'issue': '#123',
            'issues_impl': ['#456'],
            'pr_impl': ['#789'],
        },
    ]
    md = crt.generate_tracking_md(records)
    assert '| 编号 | 标题 | 状态 | 文件 | Issue | 实现 Issues | 实现 PRs |' in md
    assert '| ---' in md
    assert '001-test.md' in md
    assert 'Test RFC' in md
    assert '#123' in md
    assert '#456' in md
    assert '#789' in md
    assert '此文件由 check-rfc-tracking.py 自动生成' in md or '此文件由 check-rfc-tracking.py 自动生成' in md


def test_generate_tracking_md_minimal():
    """Records without issue/impl fields should show --."""
    records = [
        {
            'filename': '002-minimal.md',
            'title': 'Minimal',
            'state': '已接受',
            'issue': '',
            'issues_impl': [],
            'pr_impl': [],
        },
    ]
    md = crt.generate_tracking_md(records)
    assert '002-minimal.md' in md
    assert 'Minimal' in md
    assert '已接受' in md


# ── main exit code ──────────────────────────────────────────────────────

def test_main_exit_code_no_errors(monkeypatch):
    """main() should return 0 when there are no errors."""
    with tempfile.TemporaryDirectory() as tmpdir:
        draft_dir = os.path.join(tmpdir, "draft")
        os.makedirs(draft_dir)
        with open(os.path.join(draft_dir, "001-ok.md"), 'w', encoding='utf-8') as f:
            f.write("""---
title: "OK RFC"
author: "Tester"
created: "2026-07-04"
updated: "2026-07-04"
issue: "#1"
---

# OK
""")
        monkeypatch.setattr(crt, 'RFC_ROOT', tmpdir)
        monkeypatch.setattr(crt, 'TRACKING_FILE', os.path.join(tmpdir, 'TRACKING.md'))
        rc = crt.main()
        assert rc == 0


def test_main_exit_code_with_errors(monkeypatch):
    """main() should return 1 when there are errors."""
    with tempfile.TemporaryDirectory() as tmpdir:
        draft_dir = os.path.join(tmpdir, "draft")
        os.makedirs(draft_dir)
        # New file missing issue -> ERROR
        with open(os.path.join(draft_dir, "001-bad.md"), 'w', encoding='utf-8') as f:
            f.write("""---
title: "Bad RFC"
author: "Tester"
created: "2026-07-04"
updated: "2026-07-04"
---

# Bad
""")
        monkeypatch.setattr(crt, 'RFC_ROOT', tmpdir)
        monkeypatch.setattr(crt, 'TRACKING_FILE', os.path.join(tmpdir, 'TRACKING.md'))
        rc = crt.main()
        assert rc == 1


def test_main_exit_code_warnings_only(monkeypatch):
    """main() should return 0 when there are only warnings, no errors."""
    with tempfile.TemporaryDirectory() as tmpdir:
        draft_dir = os.path.join(tmpdir, "draft")
        os.makedirs(draft_dir)
        # Old file missing issue -> WARNING only
        with open(os.path.join(draft_dir, "001-old.md"), 'w', encoding='utf-8') as f:
            f.write("""---
title: "Old RFC"
author: "Tester"
created: "2025-01-01"
updated: "2025-01-01"
---

# Old
""")
        monkeypatch.setattr(crt, 'RFC_ROOT', tmpdir)
        monkeypatch.setattr(crt, 'TRACKING_FILE', os.path.join(tmpdir, 'TRACKING.md'))
        rc = crt.main()
        assert rc == 0  # Warnings don't cause exit code 1
