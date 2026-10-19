# Git 分支维护手册

> 本手册定义了 YaoXiang 项目的 Git 分支管理策略，旨在确保代码库的有序开发和高效协作。

---

## 📋 目录

- [分支类型规范](#分支类型规范)
- [命名规则](#命名规则)
- [分支生命周期](#分支生命周期)
- [工作流程](#工作流程)
- [Worktree 并行工作台](#worktree-并行工作台)
- [分支保护策略](#分支保护策略)
- [最佳实践](#最佳实践)
- [常见问题](#常见问题)

---

## 🏷️ 分支类型规范

### 核心分支（Core Branches）

| 分支名   | 用途             | 生命周期 | 保护级别 |
| -------- | ---------------- | -------- | -------- |
| `main`   | 生产环境代码     | 永久     | 严格保护 |
| `dev`    | 主开发分支       | 永久     | 中等保护 |

### 功能分支（Feature Branches）

| 前缀       | 用途             | 命名示例                                              | 合并目标       |
| ---------- | ---------------- | ----------------------------------------------------- | -------------- |
| `feature/` | 新功能开发       | `feature/type-inference`<br>`feature/ownership-model` | `dev`          |
| `bugfix/`  | 修复已知缺陷     | `bugfix/memory-leak`<br>`bugfix/parser-error`         | `dev`          |
| `hotfix/`  | 紧急线上问题修复 | `hotfix/security-patch`<br>`hotfix/crash-bug`         | `main` + `dev` |
| `release/` | 发布准备分支     | `release/v0.8.0`<br>`release/v1.0.0`                  | `main`         |

### 辅助分支（Auxiliary Branches）

| 前缀        | 用途           | 命名示例                                                   | 合并目标 |
| ----------- | -------------- | ---------------------------------------------------------- | -------- |
| `docs/`     | 文档更新       | `docs/api-reference`<br>`docs/tutorial-update`             | `dev`    |
| `ci/`       | CI/CD 配置变更 | `ci/add-deploy-script`<br>`ci/optimize-build`              | `dev`    |
| `refactor/` | 代码重构       | `refactor/lexer-optimization`<br>`refactor/memory-manager` | `dev`    |
| `test/`     | 测试相关修改   | `test/add-integration`<br>`test/performance-bench`         | `dev`    |

---

## 📝 命名规则

### 基本命名格式

```bash
# 功能分支
<type>/<short-description>

# 示例
feature/add-type-inference
bugfix/fix-parser-crash
hotfix/security-vulnerability
```

### 命名规范

1. **使用小写字母**：所有分支名使用小写
2. **使用连字符分隔**：使用 `-` 分隔单词，不使用下划线
3. **描述性命名**：分支名应清晰表达其用途
4. **避免特殊字符**：不使用空格、点号或其他特殊字符
5. **长度限制**：分支名不超过 50 个字符

### 详细示例

```bash
# ✅ 好的命名
feature/user-authentication-system
bugfix/fix-compilation-error-on-windows
hotfix/memory-leak-in-vm
docs/update-api-documentation
refactor/optimize-lexer-performance
test/add-e2e-test-cases

# ❌ 不好的命名
Feature/NewFeature  # 使用大写
bug_fix            # 使用下划线
hotfix/fix        # 描述不清晰
feature/ADD_NEW_FEATURE_WITH_LOTS_OF_DETAILS_THAT_IS_TOO_LONG  # 过长
```

---

## 🔄 分支生命周期

### 分支创建

```bash
# 1. 从最新 dev 分支创建
git checkout dev
git pull origin dev
git checkout -b feature/your-feature-name

# 2. 推送远程分支
git push -u origin feature/your-feature-name
```

### 分支开发

```bash
# 定期同步最新代码
git checkout dev
git pull origin dev
git checkout feature/your-feature-name
git rebase dev  # 或 git merge dev

# 提交代码
git add .
git commit -m ":sparkles: feat(frontend): 添加类型推断功能"
git push origin feature/your-feature-name
```

### 分支合并

```bash
# 1. 创建 Pull Request
# 2. 代码审查通过后
git checkout dev
git pull origin dev
git merge --no-ff feature/your-feature-name
git push origin dev

# 3. 清理分支
git branch -d feature/your-feature-name  # 本地删除
git push origin --delete feature/your-feature-name  # 远程删除
```

### 分支删除

```bash
# 删除已合并的功能分支
git branch -d feature/completed-feature
git push origin --delete feature/completed-feature

# 批量清理已合并分支
git branch --merged dev | grep feature | xargs -n 1 git branch -d
```

---

## 🚀 工作流程

### 功能开发流程

```mermaid
graph TD
    A[dev 分支] --> B[创建 feature 分支]
    B --> C[开发功能]
    C --> D[提交代码]
    D --> E[创建 PR 到 dev]
    E --> F[代码审查]
    F -->|通过| G[合并到 dev]
    F -->|拒绝| C
    G --> H[删除 feature 分支]
    G --> I[CI/CD 触发]
```

### 紧急修复流程

```mermaid
graph TD
    A[main 分支] --> B[创建 hotfix 分支]
    B --> C[修复问题]
    C --> D[提交代码]
    D --> E[创建 PR 到 main + dev]
    E --> F[快速审查]
    F --> G[同时合并到 main 和 dev]
    G --> H[发布 hotfix]
    I[删除 hotfix 分支]
```

### Dependabot 安全更新（恒定向 `main`，按紧急修复处理）

GitHub 的 Dependabot **安全更新**不读 `.github/dependabot.yml` 的 `target-branch`：它由**默认分支**（本仓库为 `main`）的依赖图告警驱动，PR 一律指向 `main`。每周的**版本更新** PR 才照常进 `dev`。这是 GitHub 的既有行为，不是配置写错（[dependabot-core#2767](https://github.com/dependabot/dependabot-core/issues/2767) 至今未支持改向）。

因此安全更新 PR 按上面这条流程处理，两步缺一不可：

1. 把它**合并进 `main`**。不要改基线到 `dev`——告警判定基于默认分支，改基线只会让告警一直挂着，Dependabot 还可能就同一条告警再对 `main` 开一张。
2. **当天**把 `main` 反向合并回 `dev`：

```bash
git fetch origin
git switch dev
git merge --ff-only origin/main   # dev 无独立提交时直接快进
git push origin dev
```

若 `dev` 已有独立提交无法快进，改用 `git merge origin/main`（历史形态即 `Merge remote-tracking branch 'origin/main' into dev`）。

> **适用范围**：`docs/` 这类**构建期**依赖（不进发布产物）按上面走。
> **Cargo 生产依赖**（进二进制的运行时依赖）**不适用**：等每周版本更新或下一次发版会把修复拖到一周以上，此时应在 `dev` 上人工提 PR 修，随发版进 `main`。

### 发布流程

```mermaid
graph TD
    A[dev 分支] --> B[创建 release 分支]
    B --> C[版本准备]
    C --> D[测试验证]
    D --> E[创建 PR 到 main]
    E --> F[最终审查]
    F --> G[合并到 main]
    G --> H[打版本标签]
    H --> I[合并回 dev]
    J[清理 release 分支]
```

---

## 🗂️ Worktree 并行工作台

> 适用场景：单人需要并行处理多件"必须开分支"的事，同时保持 `dev` 工作目录稳定。约定一个 worktree 作为个人并行工作台——**按需创建、用完即删**，不常驻：空置的工作台只占磁盘（本机曾有一台常驻工作台 `E:/git/YaoXiang-wt`，2026-10-15 因空置且无未合并提交而删除）。

### 约定

| 项         | 值                   | 说明                                                   |
| ---------- | -------------------- | ------------------------------------------------------ |
| 工作台路径 | `E:/git/YaoXiang-wt` | 固定路径，不带任务后缀；改名用 `git worktree move`     |
| 停车分支   | `scratch`            | 任务间的"停车位"，仅本地存在，不推送；随工作台一起删除 |
| 任务分支   | 遵循上文命名规则     | 每个任务在 worktree 里现切现用，做完删除               |

> **与 AGENTS.md 第 9 条的分工**：第 9 条约束的是**批次验证树**（唯一入口 `scripts/dev/scratch.py`，固定建在仓库内 `target/yx-scratch/<batch>/tree`，禁止成员在仓库外新建目录）；本节讲的是**人类并行的个人分支工作台**——前者服务于冻结点量测，后者服务于分支开发，用途不同，但同一个原则：**不常驻**。

### 一次任务的完整回路

```bash
# 0. 按需创建（用完即删）
git worktree add E:/git/YaoXiang-wt -b scratch dev

# 1. 领任务：在 worktree 里从最新 dev 切任务分支
cd E:/git/YaoXiang-wt
git switch -c <task-branch> dev

# 2. 开发、提交（钩子链在 worktree 里照常执行）

# 3. 合并：在【主仓库】做，不在 worktree 里做
cd E:/git/YaoXiang
git merge <task-branch>   # 小步修复直接 fast-forward，保持 dev 首父历史线性

# 4. 收尾：停回 scratch → 删已合并的任务分支 → 删工作台
cd E:/git/YaoXiang-wt
git switch scratch
cd E:/git/YaoXiang
git branch -d <task-branch>
git worktree remove E:/git/YaoXiang-wt
git branch -d scratch
```

此回路面向个人快速修复；较大变更仍走上文的 PR 流程。

### 硬性约束与坑

1. **同一分支不能被两个 worktree 同时检出**。`dev` 常驻主仓库，worktree 里 `git switch dev` 会被拒绝，只能以 `dev` 为基点切新分支——这正是需要 `scratch` 停车位的原因。
2. **合并一律在主仓库做**。主仓库持有 `dev`，是唯一合并点；worktree 只负责任务分支上的开发与提交。
3. **`target/` 构建产物不共享**。每个 worktree 独立编译（本仓库约 16 万行 Rust，首次全量重编需数分钟）；反过来说，只有「连着几天开分支干活」的时段构建缓存才持续有效，这是它主要的收益；空置时收益为零，应当及时删掉。
4. **钩子共享且正常执行**。worktree 与主仓库共享同一套钩子配置，提交时完整钩子链照常跑（历史上 worktree 的 `GIT_DIR` 泄漏问题已修复）；不要在 worktree 里执行 `pre-commit install` / `autoupdate`。
5. **路径一律写绝对路径**，Windows 下相对路径易踩坑。
6. 常用管理命令：`git worktree list` / `git worktree move <旧路径> <新路径>` / `git worktree remove <路径>`。
7. **`git worktree remove` 在 Windows 上可能留下目录残留**：装过 `node_modules` 的工作台（pnpm 用联接点）删完后，文件已清空、worktree 也已从 `.git/worktrees` 注销，但联接点骨架与空目录会留下（实测一次：残留 1238 个联接点、0 个文件、0 字节）。此时该路径已是普通目录，用 `cmd /c rd /s /q <路径>` 清掉即可；删之前先确认所有联接点的目标都落在该目录内（`Get-ChildItem -Recurse -Force` 逐个取 `.Target`），否则递归删除可能穿透到别的树。

---

## 🛡️ 分支保护策略

### 主要分支保护

**main 分支**

- 禁止直接推送
- 必须通过 PR 合并
- 禁止强制推送
- 要求代码审查
- 状态检查必须通过

**dev 分支**

- 禁止直接推送（开发成员）
- 要求 PR 合并
- 状态检查必须通过
- 允许管理员直接推送

### 分支权限设置

| 分支类型    | 开发者   | 维护者   | 管理员   |
| ----------- | -------- | -------- | -------- |
| `main`      | 仅 PR    | 仅 PR    | 审批 PR  |
| `dev`       | PR 合并  | PR 合并  | 直接推送 |
| `feature/*` | 完整权限 | 完整权限 | 完整权限 |
| `hotfix/*`  | 完整权限 | 完整权限 | 完整权限 |

---

## ✅ 最佳实践

### 1. 分支管理

- **频繁同步**：定期从 `dev` 分支获取最新代码
- **原子提交**：每个提交只包含相关变更
- **及时清理**：合并后及时删除已完成的功能分支
- **描述清晰**：分支名和提交信息要清晰表达意图

### 2. 提交规范

遵循 [提交规范](commit-convention.md)：

```bash
# 格式
:emoji: type(scope): 主题（中文）

# 示例
:sparkles: feat(frontend): 添加类型推断功能
:bug: fix(parser): 修复解析器崩溃问题
:recycle: refactor(vm): 重构虚拟机内存管理
```

### 3. Pull Request

- **描述清晰**：详细说明变更内容和原因
- **关联问题**：使用 `Closes #123` 关联相关 Issue
- **及时响应**：审查意见要及时回复
- **测试充分**：确保所有测试通过

### 4. 代码审查

- **功能正确性**：验证代码功能是否正确
- **代码质量**：检查代码是否符合规范
- **测试覆盖**：确保有适当的测试
- **文档更新**：检查文档是否需要更新

---

## ❓ 常见问题

### Q1: 如何选择分支类型？

**答：**

- 新功能 → `feature/`
- 已知缺陷修复 → `bugfix/`
- 紧急线上修复 → `hotfix/`
- 文档更新 → `docs/`
- 代码重构 → `refactor/`
- 测试相关 → `test/`

### Q2: feature 分支应该从哪个分支创建？

**答：** 始终从 `dev` 分支创建，确保功能基于最新的开发代码：

```bash
git checkout dev
git pull origin dev
git checkout -b feature/new-feature
```

### Q3: 什么时候创建 release 分支？

**答：**

- 准备发布新版本时
- 需要冻结新功能添加时
- 需要专门测试稳定版本时

### Q4: 如何处理分支冲突？

**答：**

1. 更新目标分支：`git checkout dev && git pull origin dev`
2. 切换到功能分支：`git checkout feature/your-branch`
3. 合并并解决冲突：`git rebase dev` 或 `git merge dev`
4. 解决冲突后继续开发

### Q5: hotfix 分支如何处理？

**答：**

1. 从 `main` 分支创建：`git checkout main && git checkout -b hotfix/urgent-fix`
2. 修复问题并测试
3. 同时创建 PR 到 `main` 和 `dev`
4. 合并后立即部署

### Q6: 分支命名长度有限制吗？

**答：** 建议不超过 50 个字符，保持简洁明了。Git 本身支持更长的名称，但过长的名称会影响可读性。

---

## 📚 相关文档

- [提交规范](commit-convention.md)
- [测试规范](test-specification.md)

---

## 🔧 工具和脚本

### 批量清理已合并分支

```bash
# 删除本地已合并到 dev 的分支
git checkout dev
git pull origin dev
git branch --merged dev | grep -E "^(feature|bugfix|docs|refactor|test)/" | xargs -n 1 git branch -d

# 删除远程已合并的分支
git remote prune origin
```

### 创建分支模板

```bash
#!/bin/bash
# 创建功能分支的辅助脚本

BRANCH_TYPE=$1
BRANCH_NAME=$2

if [ -z "$BRANCH_TYPE" ] || [ -z "$BRANCH_NAME" ]; then
    echo "用法: $0 <类型> <分支名>"
    echo "类型: feature, bugfix, hotfix, docs, refactor, test"
    exit 1
fi

git checkout dev
git pull origin dev
git checkout -b "$BRANCH_TYPE/$BRANCH_NAME"
git push -u origin "$BRANCH_TYPE/$BRANCH_NAME"

echo "已创建并推送分支: $BRANCH_TYPE/$BRANCH_NAME"
```

---

> 💡 **提示**：保持分支的原子性和专注性，每个分支只做一件事，这样可以让代码管理更加清晰高效！

> 📞 **支持**：如有问题，请在 GitHub Discussions 中讨论。
