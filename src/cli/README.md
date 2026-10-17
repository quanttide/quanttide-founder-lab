# qtlab-cli

量潮创始人实验室 CLI。当前为脚手架：入口与结构保留，功能待添加，参考 qtfounder 的 `src/cli`。

## 开发

```bash
cargo run              # 无子命令时输出接入提示
cargo build --release
cargo test
```

## 添加新功能

1. `src/` 下新建模块（如 `feature.rs`），在 `main.rs` 用 `mod` 声明
2. `CliCommand` 枚举注册子命令分支
3. 功能依赖加回 `Cargo.toml`

路线见 [ROADMAP.md](ROADMAP.md)。
