---
name: release
description: 发版（打 tag）完整 promoter 流程：本地门禁全量重跑、发版三查（版本号/changelog/文档生命周期）、人工批准硬停、tag 部署与验证。Use before cutting any vX.Y.Z tag or running a production deploy.
---

# Release Promoter Process（发版与打 Tag 流程规范）

## 适用场景
在执行任何 `git tag v*.*.*` 打标、合并 release 分支或触发生产部署之前，必须严格遵守本规范。

---

## 核心铁律（最高站立约束）
**未经人类用户明确书面批准，AI Agent 绝对不得执行 `git tag` 或合并发布分支。**

---

## 发版标准 4 步走 (Step-by-Step)

### 第 1 步：本地门禁全量重跑 (Local Gate Full Run)
确保工作区干净且全量门禁 100% 通过：
```bash
# 确保无未提交脏文件
git status

# 跑满门禁
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace

# 前端构建验证 (如适用)
cd admin && npm run build && cd ..
```

### 第 2 步：发版三查 (Three-Point Verification)
1. **版本号一致性**：确认 `Cargo.toml`（以及 `admin/package.json`）中的版本号已正确自增（如 `v0.1.0` -> `v0.2.0`）。
2. **变更日志 (Changelog)**：确认 `CHANGELOG.md` 或相关发布日志已记录本次版本的核心特性与破坏性变更。
3. **敏感信息与构建物扫描**：确认无私有密钥、`.env`、临时调试日志或未编译产物被包含。

### 第 3 步：人工批准硬停 (Human Approval Hard Stop)
向用户输出完整的发版摘要（包含拟定 Tag 名称、Commit Hash、变更内容清单），**明确请求人类批准**。

### 第 4 步：打 Tag 并验证 (Tag & Verification)
获得人类明确批准后，执行打标并推送到远端：
```bash
git tag -a vX.Y.Z -m "release: vX.Y.Z"
git push origin vX.Y.Z
```
