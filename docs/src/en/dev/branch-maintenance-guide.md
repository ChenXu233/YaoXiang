# Git Branch Maintenance Handbook

> This handbook defines the Git branch management strategy for the YaoXiang project, aiming to ensure orderly development and efficient collaboration of the codebase.

---

## 📋 Table of Contents

- [Branch Type Specifications](#branch-type-specifications)
- [Naming Conventions](#naming-conventions)
- [Branch Lifecycle](#branch-lifecycle)
- [Workflow](#workflow)
- [Branch Protection Strategy](#branch-protection-strategy)
- [Best Practices](#best-practices)
- [FAQ](#faq)

---

## 🏷️ Branch Type Specifications

### Core Branches

| Branch | Purpose | Lifecycle | Protection Level |
|--------|---------|-----------|------------------|
| `main` | Production environment code | Permanent | Strict protection |
| `dev` | Main development branch | Permanent | Medium protection |
| `master` | Main branch (legacy) | Permanent | Strict protection |

### Feature Branches

| Prefix | Purpose | Naming Examples | Merge Target |
|--------|---------|-----------------|--------------|
| `feature/` | New feature development | `feature/type-inference`<br>`feature/ownership-model` | `dev` |
| `bugfix/` | Fix known defects | `bugfix/memory-leak`<br>`bugfix/parser-error` | `dev` |
| `hotfix/` | Urgent production issue fix | `hotfix/security-patch`<br>`hotfix/crash-bug` | `main` + `dev` |
| `release/` | Release preparation branch | `release/v0.8.0`<br>`release/v1.0.0` | `main` |

### Auxiliary Branches

| Prefix | Purpose | Naming Examples | Merge Target |
|--------|---------|-----------------|--------------|
| `docs/` | Documentation updates | `docs/api-reference`<br>`docs/tutorial-update` | `dev` |
| `ci/` | CI/CD configuration changes | `ci/add-deploy-script`<br>`ci/optimize-build` | `dev` |
| `refactor/` | Code refactoring | `refactor/lexer-optimization`<br>`refactor/memory-manager` | `dev` |
| `test/` | Test-related modifications | `test/add-integration`<br>`test/performance-bench` | `dev` |

---

## 📝 Naming Conventions

### Basic Naming Format

```bash
# Feature branches
<type>/<short-description>

# Examples
feature/add-type-inference
bugfix/fix-parser-crash
hotfix/security-vulnerability
```

### Naming Rules

1. **Use lowercase letters**: All branch names use lowercase
2. **Use hyphens for separation**: Use `-` to separate words, not underscores
3. **Descriptive naming**: Branch names should clearly express their purpose
4. **Avoid special characters**: No spaces, dots, or other special characters
5. **Length limit**: Branch names should not exceed 50 characters

### Detailed Examples

```bash
# ✅ Good naming
feature/user-authentication-system
bugfix/fix-compilation-error-on-windows
hotfix/memory-leak-in-vm
docs/update-api-documentation
refactor/optimize-lexer-performance
test/add-e2e-test-cases

# ❌ Bad naming
Feature/NewFeature  # Uses uppercase
bug_fix            # Uses underscores
hotfix/fix        # Unclear description
feature/ADD_NEW_FEATURE_WITH_LOTS_OF_DETAILS_THAT_IS_TOO_LONG  # Too long
```

---

## 🔄 Branch Lifecycle

### Branch Creation

```bash
# 1. Create from latest dev branch
git checkout dev
git pull origin dev
git checkout -b feature/your-feature-name

# 2. Push remote branch
git push -u origin feature/your-feature-name
```

### Branch Development

```bash
# Regularly sync latest code
git checkout dev
git pull origin dev
git checkout feature/your-feature-name
git rebase dev  # or git merge dev

# Commit code
git add .
git commit -m ":sparkles: feat(frontend): add type inference feature"
git push origin feature/your-feature-name
```

### Branch Merging

```bash
# 1. Create Pull Request
# 2. After code review passes
git checkout dev
git pull origin dev
git merge --no-ff feature/your-feature-name
git push origin dev

# 3. Clean up branch
git branch -d feature/your-feature-name  # Local delete
git push origin --delete feature/your-feature-name  # Remote delete
```

### Branch Deletion

```bash
# Delete merged feature branches
git branch -d feature/completed-feature
git push origin --delete feature/completed-feature

# Batch cleanup merged branches
git branch --merged dev | grep feature | xargs -n 1 git branch -d
```

---

## 🚀 Workflow

### Feature Development Workflow

```mermaid
graph TD
    A[dev branch] --> B[Create feature branch]
    B --> C[Develop feature]
    C --> D[Commit code]
    D --> E[Create PR to dev]
    E --> F[Code review]
    F -->|Pass| G[Merge to dev]
    F -->|Reject| C
    G --> H[Delete feature branch]
    G --> I[CI/CD triggered]
```

### Urgent Fix Workflow

```mermaid
graph TD
    A[main branch] --> B[Create hotfix branch]
    B --> C[Fix issue]
    C --> D[Commit code]
    D --> E[Create PR to main + dev]
    E --> F[Quick review]
    F --> G[Merge to main and dev simultaneously]
    G --> H[Publish hotfix]
    I[Delete hotfix branch]
```

### Release Workflow

```mermaid
graph TD
    A[dev branch] --> B[Create release branch]
    B --> C[Version preparation]
    C --> D[Testing and verification]
    D --> E[Create PR to main]
    E --> F[Final review]
    F --> G[Merge to main]
    G --> H[Tag version]
    H --> I[Merge back to dev]
    J[Clean up release branch]
```

---

## 🛡️ Branch Protection Strategy

### Main Branch Protection

**main branch**
- Direct push forbidden
- Must merge via PR
- Force push forbidden
- Code review required
- Status checks must pass

**dev branch**
- Direct push forbidden (for developers)
- PR merge required
- Status checks must pass
- Administrators allowed direct push

### Branch Permission Settings

| Branch Type | Developer | Maintainer | Admin |
|------------|-----------|------------|-------|
| `main` | PR only | PR only | Approve PR |
| `dev` | PR merge | PR merge | Direct push |
| `feature/*` | Full access | Full access | Full access |
| `hotfix/*` | Full access | Full access | Full access |

---

## ✅ Best Practices

### 1. Branch Management

- **Frequent sync**: Regularly fetch latest code from `dev` branch
- **Atomic commits**: Each commit should only contain related changes
- **Timely cleanup**: Delete completed feature branches promptly after merge
- **Clear descriptions**: Branch names and commit messages should clearly express intent

### 2. Commit Convention

Follow the [commit convention](./commit-convention.md):

```bash
# Format
:emoji: type(scope): subject

# Examples
:sparkles: feat(frontend): add type inference feature
:bug: fix(parser): fix parser crash issue
:recycle: refactor(vm): refactor VM memory management
```

### 3. Pull Request

- **Clear description**: Explain the changes and reasons in detail
- **Link issues**: Use `Closes #123` to link related issues
- **Timely response**: Respond to review comments promptly
- **Sufficient testing**: Ensure all tests pass

### 4. Code Review

- **Functional correctness**: Verify the code works correctly
- **Code quality**: Check if code meets standards
- **Test coverage**: Ensure appropriate tests exist
- **Documentation updates**: Check if documentation needs updates

---

## ❓ FAQ

### Q1: How to choose a branch type?

**Answer:**
- New feature → `feature/`
- Known defect fix → `bugfix/`
- Urgent production fix → `hotfix/`
- Documentation update → `docs/`
- Code refactoring → `refactor/`
- Test-related → `test/`

### Q2: Which branch should a feature branch be created from?

**Answer:**
Always create from the `dev` branch to ensure the feature is based on the latest development code:

```bash
git checkout dev
git pull origin dev
git checkout -b feature/new-feature
```

### Q3: When to create a release branch?

**Answer:**
- When preparing to release a new version
- When freezing new feature additions
- When needing to test a stable version specifically

### Q4: How to handle branch conflicts?

**Answer:**
1. Update target branch: `git checkout dev && git pull origin dev`
2. Switch to feature branch: `git checkout feature/your-branch`
3. Merge and resolve conflicts: `git rebase dev` or `git merge dev`
4. Continue development after resolving conflicts

### Q5: How to handle hotfix branches?

**Answer:**
1. Create from `main`: `git checkout main && git checkout -b hotfix/urgent-fix`
2. Fix the issue and test
3. Create PRs to both `main` and `dev` simultaneously
4. Deploy immediately after merge

### Q6: Is there a limit on branch name length?

**Answer:**
It is recommended not to exceed 50 characters, keeping it concise and clear. Git itself supports longer names, but overly long names affect readability.

---

## 📚 Related Documents

- [Commit Convention](./commit-convention.md)
- [Code Review Guide](./code-review.md)
- [Release Guide](./release-guide.md)
- [CI/CD Configuration](../../.github/workflows/)

---

## 🔧 Tools and Scripts

### Batch Cleanup Merged Branches

```bash
# Delete local branches merged to dev
git checkout dev
git pull origin dev
git branch --merged dev | grep -E "^(feature|bugfix|docs|refactor|test)/" | xargs -n 1 git branch -d

# Delete remote merged branches
git remote prune origin
```

### Create Branch Template

```bash
#!/bin/bash
# Helper script for creating feature branches

BRANCH_TYPE=$1
BRANCH_NAME=$2

if [ -z "$BRANCH_TYPE" ] || [ -z "$BRANCH_NAME" ]; then
    echo "Usage: $0 <type> <branch-name>"
    echo "Type: feature, bugfix, hotfix, docs, refactor, test"
    exit 1
fi

git checkout dev
git pull origin dev
git checkout -b "$BRANCH_TYPE/$BRANCH_NAME"
git push -u origin "$BRANCH_TYPE/$BRANCH_NAME"

echo "Branch created and pushed: $BRANCH_TYPE/$BRANCH_NAME"
```

---

> 💡 **Tip**: Keep branches atomic and focused—each branch should do one thing—this makes code management clearer and more efficient!

> 📞 **Support**: If you have questions, please discuss in GitHub Discussions.