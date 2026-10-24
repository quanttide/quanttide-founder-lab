//! qtfounder-lab：量潮创始人实验室 CLI（包名 `qtfounder-cli-lab`，命令 `qtfounder-lab`，与正式版 `qtfounder` 区分）。
//!
//! 两个入口，名词读、动词写：
//! - `qtfounder-lab product requirement`：看有哪些需求、哪些差重复了（只读）
//! - `qtfounder-lab product clarify`：澄清一次（唯一写入路径）
//!
//! 契约见 `docs/api-references/`：字段、flag、JSON 出入、错误 JSON、退出码三档（0/2/3）。

mod store;
mod product;

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{CommandFactory, Parser, Subcommand};
use serde::Serialize;

/// 统一错误：错误一律 JSON 写 stderr，退出码三档。
#[derive(Debug)]
pub enum AppError {
    /// 用法错（参数缺失、互斥、类型不对、读写失败）→ 退出码 2
    Usage(String),
    /// 未找到（`--id` 指向的需求不存在）→ 退出码 3
    NotFound { message: String, hint: String },
}

impl AppError {
    pub fn usage(message: impl Into<String>) -> Self {
        AppError::Usage(message.into())
    }
}

/// 错误信封：`{"error":{...}}`；`hint` 仅 NOT_FOUND 带。
#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    error: ErrorPayload<'a>,
}

#[derive(Serialize)]
struct ErrorPayload<'a> {
    code: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'a str>,
}

#[derive(Parser)]
#[command(name = "qtfounder-lab", about = "量潮创始人实验室 CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,
}

/// 顶层子命令注册表：新功能在这里加一个枚举分支。
#[derive(Subcommand)]
enum CliCommand {
    /// 需求与澄清两个入口：名词读、动词写
    Product {
        #[command(subcommand)]
        command: ProductCommand,
    },
}

#[derive(Subcommand)]
pub enum ProductCommand {
    /// 看有哪些需求、哪些差重复了（只读）
    Requirement {
        /// 给定时查一件事的详情，不给则列出全部
        #[arg(long)]
        id: Option<String>,
        /// 输出机器可读 JSON
        #[arg(long)]
        json: bool,
    },
    /// 澄清一次：记录一次「不对劲」，当场给出一段能粘回对话的话
    Clarify {
        /// 新开一件事；不给则挂到最近一次需求上
        #[arg(long)]
        task: Option<String>,
        /// 整条 JSON 从标准输入读入，字段名与 --json 输出一致
        #[arg(long)]
        stdin: bool,
        /// 修正一条已记录的澄清（给记录 id），原地改并重拼提示词
        #[arg(long)]
        revise: Option<String>,
        /// 把候选固化成一条规则，追加进 rules.md
        #[arg(long)]
        rule: Option<String>,
        /// 输出机器可读 JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => exit_with_clap_error(err),
    };
    let outcome = match cli.command {
        Some(CliCommand::Product { command }) => product::run(command),
        None => {
            let _ = Cli::command().print_help();
            Ok(())
        }
    };
    if let Err(err) = outcome {
        print_error(&err);
        std::process::exit(match err {
            AppError::Usage(_) => 2,
            AppError::NotFound { .. } => 3,
        });
    }
}

/// clap 自身的解析错误统一改写成本 CLI 的错误 JSON（help/version 除外）。
fn exit_with_clap_error(err: clap::Error) -> ! {
    match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            let _ = err.print();
            std::process::exit(err.exit_code());
        }
        ErrorKind::UnknownArgument => {
            let unknown = match err.get(ContextKind::InvalidArg) {
                Some(ContextValue::String(value)) => value.as_str(),
                _ => "",
            };
            print_error(&AppError::Usage(format!("未知参数 {unknown}")));
            std::process::exit(2);
        }
        ErrorKind::MissingSubcommand | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            print_error(&AppError::usage("缺少子命令：requirement 或 clarify"));
            std::process::exit(2);
        }
        _ => {
            let text = err.to_string();
            let message = text.lines().next().unwrap_or("用法错误");
            print_error(&AppError::usage(message));
            std::process::exit(2);
        }
    }
}

fn print_error(err: &AppError) {
    let envelope = ErrorEnvelope {
        error: match err {
            AppError::Usage(message) => ErrorPayload {
                code: "USAGE",
                message,
                hint: None,
            },
            AppError::NotFound { message, hint } => ErrorPayload {
                code: "NOT_FOUND",
                message,
                hint: Some(hint),
            },
        },
    };
    eprintln!(
        "{}",
        serde_json::to_string(&envelope).expect("错误 JSON 序列化不会失败")
    );
}
