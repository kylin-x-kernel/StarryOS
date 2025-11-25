# StarryOS 代码统计

##  功能说明

 自动统计 StarryOS 及其依赖仓库的代码量，包括：
- 主仓库（StarryOS）
- local_crates（axplat_crates, axdriver_crates, axcpu 等）
- 白名单中的仓库

统计工具基于 [starry-test-harness](https://github.com/kylin-x-kernel/starry-test-harness) 的代码统计功能。

##  运行时机

- **自动运行**：每天凌晨 2:00 UTC（北京时间 10:00）
- **手动触发**：在 Actions 页面随时运行

##  查看统计结果

### 方式 1：查看结果分支（推荐）

统计结果自动推送到独立分支：

**最新报告**：
```
https://github.com/kylin-x-kernel/StarryOS/blob/code-stats-results/reports/latest.md
```

**历史记录**：
```
https://github.com/kylin-x-kernel/StarryOS/tree/code-stats-results/reports
```

结构：
```
code-stats-results/
└── reports/
    ├── README.md              # 报告索引
    ├── latest.json            # 最新统计（JSON）
    ├── latest.md              # 最新统计（Markdown）
    ├── latest-info.json       # 元信息（时间戳、commit）
    └── 20251125-020000/       # 历史报告
        ├── loc.json
        └── loc.md
```

### 方式 2：下载 Artifact

直接在线看

### 方式 3：查看 Actions Summary

在 Actions 运行详情的 Summary 标签页直接查看完整报告。

##  手动触发

1. 进入 Actions 页面
2. 点击 "Run workflow"
3. 点击绿色的 "Run workflow" 按钮

##  统计内容

### 统计指标

- 各仓库总代码行数（LOC）
- 语言分布及占比
- 仓库排行（按代码量降序）

### 输出格式

- **loc.json**：JSON 格式详细数据
- **loc.md**：Markdown 格式报告

##  修改统计配置

如需修改统计的仓库列表，编辑 `.github/workflows/code-stats.yml` 中的白名单配置：直接往repos下面加仓库地址，现在branch写死的main分支，可以改。

```yaml
- name: Create whitelist config
  run: |
    cat > .code-stats-whitelist.yml << 'EOF'
    branch: main
    repos:


      # 添加更多仓库...
    EOF
```

##  故障排除

### CI 运行失败

1. 检查 Actions 日志
2. 确认依赖仓库可访问
3. 验证白名单中的仓库分支是否正确

### 结果分支不存在

首次运行会自动创建 `code-stats-results` 分支，如果失败请检查仓库权限。

##  相关链接

- [starry-test-harness 仓库](https://github.com/kylin-x-kernel/starry-test-harness)
- [tokei 文档](https://github.com/XAMPPRocky/tokei)
- [Actions 工作流配置](../.github/workflows/code-stats.yml)