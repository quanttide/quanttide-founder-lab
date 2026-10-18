//! 两个入口的实现：`work intent`（只读）、`work clarify`（唯一写入路径）。

use std::io::{self, Read as _, Write as _};

use serde::Serialize;
use serde_json::Value;

use crate::store::{
    Clarification, Intent, Store, append_rule_line, dedup_key, new_id, now_iso8601,
};
use crate::{AppError, WorkCommand};

pub fn run(command: WorkCommand) -> Result<(), AppError> {
    match command {
        WorkCommand::Intent { id, json } => intent(id, json),
        WorkCommand::Clarify {
            task,
            stdin,
            rule,
            json,
        } => clarify(task, stdin, rule, json),
    }
}

// ---------- intent（名词，只读） ----------

#[derive(Serialize)]
struct IntentList {
    intents: Vec<IntentSummary>,
}

#[derive(Serialize)]
struct IntentSummary {
    id: String,
    task: String,
    at: String,
    clarifications: usize,
    candidates: Vec<CandidateSummary>,
}

#[derive(Serialize)]
struct CandidateSummary {
    actual_key: String,
    count: usize,
    clarification_ids: Vec<String>,
}

#[derive(Serialize)]
struct IntentDetail<'a> {
    id: &'a str,
    task: &'a str,
    at: &'a str,
    clarifications: &'a [Clarification],
}

fn intent(id: Option<String>, json: bool) -> Result<(), AppError> {
    let store = Store::load()?;
    match id {
        None => list_intents(&store, json),
        Some(id) => show_intent(&store, &id, json),
    }
}

fn list_intents(store: &Store, json: bool) -> Result<(), AppError> {
    let summaries: Vec<IntentSummary> = store
        .intents
        .iter()
        .map(|intent| IntentSummary {
            id: intent.id.clone(),
            task: intent.task.clone(),
            at: intent.at.clone(),
            clarifications: intent.clarifications.len(),
            candidates: intent
                .candidates()
                .into_iter()
                .map(|group| CandidateSummary {
                    actual_key: group.key,
                    count: group.ids.len(),
                    clarification_ids: group.ids,
                })
                .collect(),
        })
        .collect();

    if json {
        let list = IntentList { intents: summaries };
        println!("{}", to_pretty(&list)?);
        return Ok(());
    }

    if summaries.is_empty() {
        println!("（暂无意图，先跑 qtfounder-lab work clarify）");
        return Ok(());
    }
    for summary in &summaries {
        println!(
            "{}  {}  {} 次澄清",
            summary.id, summary.task, summary.clarifications
        );
        for candidate in &summary.candidates {
            println!("  候选：{}  ×{}", candidate.actual_key, candidate.count);
        }
    }
    Ok(())
}

fn show_intent(store: &Store, id: &str, json: bool) -> Result<(), AppError> {
    let intent = store
        .intents
        .iter()
        .find(|intent| intent.id == id)
        .ok_or_else(|| AppError::NotFound {
            message: format!("{id} 不存在"),
            hint: "用 qtfounder-lab work intent 查看".to_string(),
        })?;

    if json {
        let detail = IntentDetail {
            id: &intent.id,
            task: &intent.task,
            at: &intent.at,
            clarifications: &intent.clarifications,
        };
        println!("{}", to_pretty(&detail)?);
        return Ok(());
    }

    println!("{}  {}  {}", intent.id, intent.task, intent.at);
    for cl in &intent.clarifications {
        println!("  {}  {}", cl.id, cl.at);
        println!("    原以为：{}", cl.before);
        println!("    实际：{}", cl.actual);
        println!("    提示词：{}", cl.prompt);
    }
    Ok(())
}

// ---------- clarify（动词，唯一写入路径） ----------

#[derive(Serialize)]
struct ClarifyOutcome {
    id: String,
    intent_id: String,
    at: String,
    deduplicated: bool,
    prompt: String,
}

#[derive(Serialize)]
struct RuleOutcome {
    rule: String,
    source: Vec<String>,
    path: String,
}

/// 一次澄清的输入，交互与 `--stdin` 汇到同一个形状。
struct ClarifyInput {
    task: Option<String>,
    before: String,
    actual: String,
    prompt: String,
}

fn clarify(
    task: Option<String>,
    from_stdin: bool,
    rule: Option<String>,
    json: bool,
) -> Result<(), AppError> {
    if let Some(rule) = rule {
        if task.is_some() {
            return Err(AppError::usage("--task 与 --rule 不能同时使用"));
        }
        let rule = rule.trim();
        if rule.is_empty() {
            return Err(AppError::usage("--rule 需要一段文本"));
        }
        // `--rule` 与 `--stdin` 同时给时只立规则，不读 stdin。
        return append_rule(rule, json);
    }

    let mut store = Store::load()?;
    let input = if from_stdin {
        input_from_stdin(task)?
    } else {
        input_interactive(task, &store)?
    };

    let key = dedup_key(&input.before, &input.actual);
    let existing = store
        .find_by_dedup_key(&key)
        .map(|(intent_id, cl)| (intent_id, cl.clone()));

    let outcome = match existing {
        Some((intent_id, cl)) => ClarifyOutcome {
            id: cl.id,
            intent_id,
            at: cl.at,
            deduplicated: true,
            prompt: cl.prompt,
        },
        None => {
            let intent_id = match input.task {
                Some(task) => {
                    let id = new_id("int", |candidate| store.intent_exists(candidate));
                    store.intents.push(Intent {
                        id: id.clone(),
                        task,
                        at: now_iso8601(),
                        clarifications: Vec::new(),
                    });
                    id
                }
                None => store
                    .intents
                    .last()
                    .map(|intent| intent.id.clone())
                    .ok_or_else(|| AppError::usage("没有可挂的意图，缺少 task"))?,
            };
            let id = new_id("clr", |candidate| {
                store.intent_exists(candidate) || store.clarification_exists(candidate)
            });
            let at = now_iso8601();
            let intent = store
                .intents
                .iter_mut()
                .find(|intent| intent.id == intent_id)
                .expect("目标意图刚创建或已存在");
            intent.clarifications.push(Clarification {
                id: id.clone(),
                at: at.clone(),
                before: input.before,
                actual: input.actual,
                prompt: input.prompt.clone(),
            });
            store.save()?;
            ClarifyOutcome {
                id,
                intent_id,
                at,
                deduplicated: false,
                prompt: input.prompt,
            }
        }
    };

    if json {
        println!("{}", to_pretty(&outcome)?);
    } else {
        println!("{}", outcome.prompt);
        println!();
        if outcome.deduplicated {
            println!(
                "已存在 {} / {}（重复提交，未新增记录）",
                outcome.intent_id, outcome.id
            );
        } else {
            println!("已记入 {} / {}", outcome.intent_id, outcome.id);
        }
    }
    Ok(())
}

/// `--stdin`：整条 JSON 读入，字段名与输出一致。
fn input_from_stdin(flag_task: Option<String>) -> Result<ClarifyInput, AppError> {
    let mut text = String::new();
    io::stdin()
        .read_to_string(&mut text)
        .map_err(|err| AppError::usage(format!("读取标准输入失败：{err}")))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|err| AppError::usage(format!("JSON 解析失败：第 {} 行", err.line())))?;

    let task = match flag_task {
        Some(task) => Some(task),
        None => optional_text(&value, "task")?,
    };
    Ok(ClarifyInput {
        task,
        before: required_text(&value, "before")?,
        actual: required_text(&value, "actual")?,
        prompt: required_text(&value, "prompt")?,
    })
}

/// 交互：需要新开时先问任务，再问两问；答得笼统时各追问一次。
fn input_interactive(flag_task: Option<String>, store: &Store) -> Result<ClarifyInput, AppError> {
    let task = match flag_task {
        Some(task) => Some(task),
        None if store.intents.is_empty() => Some(ask_task()?),
        None => None,
    };
    let before = ask_question("你原本以为它知道 / 会做什么？")?;
    let actual = ask_question("它实际做了什么？")?;
    let prompt = template_prompt(&before, &actual);
    Ok(ClarifyInput {
        task,
        before,
        actual,
        prompt,
    })
}

/// 交互产出的模板提示词（`--stdin` 的 prompt 由输入给）。
fn template_prompt(before: &str, actual: &str) -> String {
    format!(
        "我原以为「{before}」。\n它实际做了「{actual}」。\n请按我原以为的重做一遍，把这两条当约束。"
    )
}

fn ask_task() -> Result<String, AppError> {
    print!("任务：");
    flush()?;
    let text = read_line()?;
    println!();
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::usage("缺少回答"));
    }
    Ok(text.to_string())
}

fn ask_question(question: &str) -> Result<String, AppError> {
    println!("{question}");
    let answer = ask_line()?;
    if answer.chars().count() < 6 {
        // 答得笼统：追问一次，概括是情绪，不是信息
        println!("举它刚才那句话或那个动作？");
        return ask_line();
    }
    Ok(answer)
}

fn ask_line() -> Result<String, AppError> {
    print!("> ");
    flush()?;
    let text = read_line()?;
    println!();
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::usage("缺少回答"));
    }
    Ok(text.to_string())
}

fn read_line() -> Result<String, AppError> {
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|err| AppError::usage(format!("读取回答失败：{err}")))?;
    if line.is_empty() {
        return Err(AppError::usage("缺少回答"));
    }
    Ok(line)
}

fn flush() -> Result<(), AppError> {
    io::stdout()
        .flush()
        .map_err(|err| AppError::usage(format!("输出失败：{err}")))
}

fn required_text(value: &Value, field: &str) -> Result<String, AppError> {
    match value.get(field) {
        None | Some(Value::Null) => Err(AppError::usage(format!("缺少字段 {field}"))),
        Some(Value::String(text)) if text.trim().is_empty() => {
            Err(AppError::usage(format!("缺少字段 {field}")))
        }
        Some(Value::String(text)) => Ok(text.trim().to_string()),
        Some(_) => Err(AppError::usage(format!("字段 {field} 必须是文本"))),
    }
}

fn optional_text(value: &Value, field: &str) -> Result<Option<String>, AppError> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if text.trim().is_empty() => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.trim().to_string())),
        Some(_) => Err(AppError::usage(format!("字段 {field} 必须是文本"))),
    }
}

/// 立规则：追加进 rules.md，并把重复的差标为已处理。
fn append_rule(rule: &str, json: bool) -> Result<(), AppError> {
    let mut store = Store::load()?;
    let source = store.unprocessed_duplicate_ids();
    let path = append_rule_line(rule)?;
    store
        .processed_clarifications
        .extend(source.iter().cloned());
    store.save()?;

    if json {
        let outcome = RuleOutcome {
            rule: rule.to_string(),
            source,
            path: path.display().to_string(),
        };
        println!("{}", to_pretty(&outcome)?);
    } else {
        println!("已追加规则到 {}", path.display());
        println!("- {rule}");
        if source.is_empty() {
            println!("没有新的重复差。");
        } else {
            println!("已处理 {} 条重复差：{}", source.len(), source.join(", "));
        }
    }
    Ok(())
}

fn to_pretty<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string_pretty(value)
        .map_err(|err| AppError::usage(format!("输出序列化失败：{err}")))
}
