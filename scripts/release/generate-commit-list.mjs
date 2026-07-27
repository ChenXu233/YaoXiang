#!/usr/bin/env node
/**
 * Generate commit list with @author mentions for release body.
 *
 * Usage:
 *   node scripts/release/generate-commit-list.mjs \
 *     --from-tag v0.7.9 --to-tag v0.7.10 \
 *     --changelog CHANGELOG.md
 *
 * Output:
 *   Full release body (CHANGELOG minus old commit list + new commit list) to stdout.
 *
 * GitHub username resolution:
 *   - noreply email (ID+user@users.noreply.github.com) → parse directly
 *   - Other emails → look up via GitHub API, cached in-memory
 */

import { readFileSync } from "node:fs";
import { execSync } from "node:child_process";

const GITHUB_NOREPLY_RE = /^\d+\+(.+)@users\.noreply\.github\.com$/;

function parseArgs() {
  const args = process.argv.slice(2);
  const opts = {};
  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case "--from-tag":
        opts.fromTag = args[++i];
        break;
      case "--to-tag":
        opts.toTag = args[++i];
        break;
      case "--changelog":
        opts.changelog = args[++i];
        break;
      case "--help":
      case "-h":
        console.log(`Usage: node scripts/release/generate-commit-list.mjs \\
  --from-tag <tag> --to-tag <tag> \\
  --changelog <path>`);
        process.exit(0);
    }
  }
  if (!opts.fromTag || !opts.toTag || !opts.changelog) {
    console.error("Error: --from-tag, --to-tag, and --changelog are required");
    process.exit(1);
  }
  return opts;
}

/**
 * Parse GitHub username from noreply email, e.g.:
 *   91937041+ChenXu233@users.noreply.github.com → ChenXu233
 *   49699333+dependabot[bot]@users.noreply.github.com → dependabot[bot]
 * Returns null if not a noreply email.
 */
function parseNoreplyUsername(email) {
  const match = email.match(GITHUB_NOREPLY_RE);
  return match ? match[1] : null;
}

/**
 * Resolve a git email to a GitHub username via API.
 * Uses in-memory cache to avoid duplicate lookups.
 */
class UsernameResolver {
  constructor() {
    this.cache = new Map(); // email → username | null
  }

  async resolve(email) {
    if (this.cache.has(email)) return this.cache.get(email);

    // 1. Try noreply parsing first
    const fromNoreply = parseNoreplyUsername(email);
    if (fromNoreply) {
      this.cache.set(email, fromNoreply);
      return fromNoreply;
    }

    // 2. Try GitHub API search
    try {
      const result = execSync(
        `gh api "search/users?q=${encodeURIComponent(email)}+in:email" --jq ".items[0].login"`,
        { encoding: "utf-8", timeout: 10000 }
      ).trim();
      const username = result || null;
      this.cache.set(email, username);
      return username;
    } catch {
      this.cache.set(email, null);
      return null;
    }
  }
}

function getCommits(fromTag, toTag) {
  const output = execSync(
    `git log "${fromTag}..${toTag}" --no-merges --format="%H|||%an|||%ae|||%s"`,
    { encoding: "utf-8", maxBuffer: 10 * 1024 * 1024 }
  );
  return output
    .trim()
    .split("\n")
    .filter(Boolean)
    .map((line) => {
      const parts = line.split("|||", 4);
      return { hash: parts[0], author: parts[1], email: parts[2], subject: parts[3] };
    });
}

async function generateCommitTable(commits, resolver) {
  const warned = new Set();
  const lines = [
    "### 📝 提交记录",
    "",
    `共 ${commits.length} 个 commit（无 merge）`,
    "",
    "| 作者 | Hash | 描述 |",
    "| :---: | :---: | --- |",
  ];

  for (const { hash, author, email, subject } of commits) {
    const username = await resolver.resolve(email);
    let authorCell;
    if (username) {
      authorCell = `@${username}`;
    } else {
      authorCell = author;
      if (!warned.has(email)) {
        console.warn(`::warning::No GitHub username found for email <${email}> (author: ${author}), using raw name`);
        warned.add(email);
      }
    }
    const shortHash = hash.slice(0, 7);
    const safeSubject = subject.replace(/\|/g, "\\|");
    lines.push(`| ${authorCell} | \`${shortHash}\` | ${safeSubject} |`);
  }

  return lines.join("\n") + "\n";
}

function stripOldCommitSection(content) {
  const marker = "### 📝 提交记录";
  const idx = content.indexOf(marker);
  if (idx !== -1) {
    return content.slice(0, idx).trimEnd() + "\n";
  }
  return content;
}

async function main() {
  const args = parseArgs();

  const changelog = readFileSync(args.changelog, "utf-8");
  const body = stripOldCommitSection(changelog);

  const commits = getCommits(args.fromTag, args.toTag);
  if (commits.length === 0) {
    console.error("::error::No commits found between tags");
    process.exit(1);
  }

  const resolver = new UsernameResolver();
  const commitTable = await generateCommitTable(commits, resolver);

  process.stdout.write(body);
  process.stdout.write("\n");
  process.stdout.write(commitTable);
}

main().catch((err) => {
  console.error("::error::", err.message);
  process.exit(1);
});