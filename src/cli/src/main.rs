//! qtlab-cli：量潮创始人实验室 CLI。
//!
//! 当前为脚手架：入口与结构保留，功能待添加。参考 qtfounder 的 `src/cli`。
//!
//! 添加新功能三步：
//! 1. 在 `src/` 下新建模块（如 `feature.rs`），用 `mod` 声明
//! 2. 在 `CliCommand` 枚举里注册一个子命令分支
//! 3. 把该功能的依赖加回 `Cargo.toml`

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "qtlab", about = "量潮创始人实验室 CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,
}

/// 子命令注册表：新功能在这里加一个枚举分支。
#[derive(Subcommand)]
enum CliCommand {}

fn main() {
    match Cli::parse().command {
        Some(cmd) => match cmd {},
        None => {
            println!("qtlab：暂无子命令。");
            println!("新功能在 src/ 下新增模块，并在 CliCommand 注册。");
        }
    }
}
