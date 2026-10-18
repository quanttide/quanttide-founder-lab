# qtfounder-cli-lab

量潮创始人实验室 CLI（命令名 `qtfounder-lab`，与正式版 `qtfounder` 区分），已实现两个入口：名词读的 `work intent`、动词写的 `work clarify`。

## 开发

```bash
cargo run -- work clarify     # 交互澄清一次（无子命令时输出帮助）
cargo run -- work intent      # 列意图与候选
cargo build --release
cargo test                    # 离线环境用 cargo test --offline
```

## 存储

单文件 JSON `data/clarify.json`（环境变量 `QTFOUNDER_LAB_DATA` 覆盖整个数据目录），规则在同目录 `rules.md`；`data/` 已在 `.gitignore`。

## 添加新功能

1. `src/` 下新建模块（如 `feature.rs`），在 `main.rs` 用 `mod` 声明
2. `CliCommand` 枚举注册子命令分支
3. 功能依赖加回 `Cargo.toml`（只用本地 cargo 缓存已有的，离线加 `--offline`）

路线见 [ROADMAP.md](ROADMAP.md)，命令行接口（两个入口、入参、JSON 出入、退出码）见 [docs/api-references/](docs/api-references/index.md)，实现报告与测试覆盖表见 [REPORT.md](REPORT.md)。
