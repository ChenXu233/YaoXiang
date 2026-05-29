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

| Branch Name | Purpose | Lifecycle | Protection Level |
|-------------|---------|-----------|------------------|
| `main` | Production environment code | Permanent | Strict protection |
| `dev` | Main development branch | Permanent | Medium protection |
| `master` | Main trunk branch (legacy) | Permanent | Strict protection |

### Feature Branches

| Prefix | Purpose | Naming Examples | Merge Target |
|--------|---------|------------------|--------------|
| `feature/` | New feature development | `feature/type-inference`<br>`feature/ownership-model` | `dev` |
| `bugfix/` | Fix known defects | `bugfix/memory-leak`<br>`bugfix/parser-error` | `dev` |
| `hotfix/` | Emergency production fix | `hotfix/security-patch`<br>`hotfix/crash-bug` | `main` + `dev` |
| `release/` | Release preparation branch | `release/v0.8.0`<br>`release/v1.0.0` | `main` |

### Auxiliary Branches

| Prefix | Purpose | Naming Examples | Merge Target |
|--------|---------|------------------|--------------|
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

### Naming Guidelines

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
bug_fix            # Uses underscore
hotfix/fix        # Unclear description
feature/ADD_NEW_FEATURE_WITH_LOTS_OF_DETAILS_THAT_IS_TOO_LONG  # Too long
```

---

## 🔄 Branch Lifecycle

### Creating a Branch

```bash
# 1. Create from the latest dev branch
git checkout dev
git pull origin dev
git checkout -b feature/your-feature-name

# 2. Push to remote branch
git push -u origin feature/your-feature-name
```

### Branch Development

```bash
# Regularly sync the latest code
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
# 1. Create a Pull Request
# 2. After code review passes
git checkout dev
git pull origin dev
git merge --no-ff feature/your-feature-name
git push origin dev

# 3. Clean up branch
git branch -d feature/your-feature-name  # Local deletion
git push origin --delete feature/your-feature-name  # Remote deletion
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

### Emergency Fix Workflow

```mermaid
graph TD
    A[main branch] --> B[Create hotfix branch]
    B --> C[Fix issue]
    C --> D[Commit code]
    D --> E[Create PR to main + dev]
    E --> F[Quick review]
    F --> G[Merge to main and dev simultaneously]
    G --> H[Release hotfix]
    I[Delete hotfix branch]
```

### Release Workflow

```mermaid
graph TD
    A[dev branch] --> B[Create release branch]
    B --> C[Version preparation]
    C --> D[Test and verify]
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
- Direct push prohibited
- Must merge via PR
- Force push prohibited
- Code review required
- Status checks must pass

**dev branch**
- Direct push prohibited (for developers)
- PR merge required
- Status checks must pass
- Admins allowed direct push

### Branch Permission Settings

| Branch Type | Developer | Maintainer | Admin |
|-------------|-----------|------------|-------|
| `main` | PR only | PR only | Approve PR |
| `dev` | PR merge | PR merge | Direct push |
| `feature/*` | Full permissions | Full permissions | Full permissions |
| `hotfix/*` | Full permissions | Full permissions | Full permissions |

---

## ✅ Best Practices

### 1. Branch Management

- **Frequent sync**: Regularly fetch latest code from `dev` branch
- **Atomic commits**: Each commit only contains related changes
- **Timely cleanup**: Delete completed feature branches after merging
- **Clear descriptions**: Branch names and commit messages should clearly express intent

### 2. Commit Conventions

Follow the [commit conventions](./commit-convention.md):

```bash
# Format
:emoji: type(scope): subject

# Examples
:sparkles: feat(frontend): add type inference feature
:bug: fix(parser): fix parser crash issue
:recycle: refactor(vm): refactor VM memory management
```

### 3. Pull Request

- **Clear description**: Explain changes and reasons in detail
- **Link issues**: Use `Closes #123` to link related Issues
- **Respond promptly**: Reply to review comments in a timely manner
- **Test thoroughly**: Ensure all tests pass

### 4. Code Review

- **Functional correctness**: Verify if the code functions correctly
- **Code quality**: Check if code conforms to standards
- **Test coverage**: Ensure appropriate tests are in place
- **Documentation updates**: Check if documentation needs updating

---

## ❓ FAQ

### Q1: How to choose a branch type?

**Answer:**
- New feature → `feature/`
- Known defect fix → `bugfix/`
- Emergency production fix → `hotfix/`
- Documentation update → `docs/`
- Code refactoring → `refactor/`
- Test related → `test/`

### Q2: From which branch should I create a feature branch?

**Answer:**
Always create from the `dev` branch, ensuring the feature is based on the latest development code:

```bash
git checkout dev
git pull origin dev
git checkout -b feature/new-feature
```

### Q3: When should I create a release branch?

**Answer:**
- When preparing to release a new version
- When freezing new feature additions is needed
- When dedicated testing of a stable version is required

### Q4: How to handle branch conflicts?

**Answer:**
1. Update the target branch: `git checkout dev && git pull origin dev`
2. Switch to the feature branch: `git checkout feature/your-branch`
3. Merge and resolve conflicts: `git rebase dev` or `git merge dev`
4. Continue development after resolving conflicts

### Q5: How to handle hotfix branches?

**Answer:**
1. Create from `main` branch: `git checkout main && git checkout -b hotfix/urgent-fix`
2. Fix the issue and test
3. Create PRs to both `main` and `dev` simultaneously
4. Deploy immediately after merging

### Q6: Is there a limit on branch name length?

**Answer:**
It is recommended to not exceed 50 characters, keeping it concise and clear. Git itself supports longer names, but overly long names affect readability.

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
    echo "Types: feature, bugfix, hotfix, docs, refactor, test"
    exit 1
fi

git checkout dev
git pull origin dev
git checkout -b "$BRANCH_TYPE/$BRANCH_NAME"
git push -u origin "$BRANCH_TYPE/$BRANCH_NAME"

echo "Branch created and pushed: $BRANCH_TYPE/$BRANCH_NAME"
```

---

> 💡 **Tip**: Keep branches atomic and focused, each branch should do one thing, this makes code management clearer and more efficient!

> 📞 **Support**: If you have questions, please discuss in GitHub Discussions.