//! `health reflect`：一条反思的 CBT 记录。
//!
//! 记的不是情绪，是一条可检验的判断链：情境 → 自动思维 → 认知扭曲 → 正反证据 →
//! 替代思维 → 行为实验。角色归属（`company`/`self`、`scarce`/`grasp`）接
//! founder-reflect 的两问，行为实验接它的「放手实验」。
//!
//! 三件事：记一条（`--stdin`）、看还差什么与到期待复看（`--list`）、
//! 回填实验结果（`--observe`）。规则不在这里落——反思给出的是一句话判断，
//! 攒够了由人自己写进规则文件。

use std::fs;
use std::io::{self, Read as _};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::store::{data_dir, new_id, now_iso8601};
use crate::{AppError, HealthCommand};

const DATA_FILE: &str = "reflect.json";

/// 认知扭曲词表：英文键与中文标签互为别名，落库统一用中文。
const DISTORTIONS: &[(&str, &str)] = &[
    ("all_or_nothing", "全或无"),
    ("catastrophizing", "灾难化"),
    ("mind_reading", "读心"),
    ("should", "应该式"),
    ("personalization", "个人化"),
    ("emotional_reasoning", "情绪化推理"),
    ("discounting_positive", "贬低积极面"),
    ("overgeneralization", "过度概括"),
    ("labeling", "贴标签"),
    ("mental_filter", "心理过滤"),
];

/// 归属：这件事属于公司，还是属于我这个人。
const DOMAINS: &[&str] = &["company", "self"];
/// 控制：结构性稀缺（真的只有我能做）还是心理性抓取（不放心）。
const CONTROLS: &[&str] = &["scarce", "grasp"];

/// 还缺哪一栏、下一问是什么，按 CBT 的填栏顺序。
const QUESTIONS: &[(&str, &str)] = &[
    ("emotion", "这件事让你感到什么？强度大概几分（0-100）？"),
    (
        "distortions",
        "这个念头属于哪种思维陷阱？全或无、灾难化、读心、应该式、个人化、情绪化推理、贬低积极面、过度概括",
    ),
    ("evidence_against", "有什么事实说明这件事没你想的那么糟？"),
    ("balanced", "如果朋友遇到同样的事，你会怎么对他说？"),
    (
        "experiment",
        "打算怎么验证这个判断？给一个预测和一个复看日期。",
    ),
];

const COMPLETE_QUESTION: &str = "到复看日回填结果：实际发生的事，比任何推理都算数。";

/// 人读时最多列几条未填完的记录：欠账全摊出来，列表就成了新的负担。
const OPEN_SHOWN: usize = 3;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Ledger {
    #[serde(default)]
    reflections: Vec<Reflection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Reflection {
    id: String,
    at: String,
    situation: String,
    thought: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    emotion: Option<Emotion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    distortions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    evidence_for: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    evidence_against: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    balanced: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    control: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    experiment: Option<Experiment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Emotion {
    label: String,
    intensity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Experiment {
    prediction: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    review_on: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    emotion_after: Option<u8>,
}

pub fn run(command: HealthCommand) -> Result<(), AppError> {
    match command {
        HealthCommand::Reflect {
            stdin,
            list,
            due,
            id,
            observe,
            result,
            emotion,
            json,
        } => reflect(stdin, list, due, id, observe, result, emotion, json),
    }
}

#[allow(clippy::too_many_arguments)]
fn reflect(
    stdin: bool,
    list: bool,
    due: bool,
    id: Option<String>,
    observe: Option<String>,
    result: Option<String>,
    emotion_after: Option<u8>,
    json: bool,
) -> Result<(), AppError> {
    let modes = [stdin, list, id.is_some(), observe.is_some()];
    if modes.iter().filter(|on| **on).count() != 1 {
        return Err(AppError::usage(
            "四选一：--stdin（记一条）、--list（看待办与重复）、--id（看一条）、--observe（回填结果）",
        ));
    }
    if due && !list {
        return Err(AppError::usage("--due 只与 --list 一起用"));
    }
    if result.is_some() && observe.is_none() {
        return Err(AppError::usage("--result 只与 --observe 一起用"));
    }
    if emotion_after.is_some() && observe.is_none() {
        return Err(AppError::usage("--emotion 只与 --observe 一起用"));
    }

    if list {
        return print_list(due, json);
    }
    if stdin {
        return record(json);
    }
    if let Some(id) = id {
        return print_one(&id, json);
    }
    let observe = observe.expect("前面的模式检查已保证 --observe 存在");
    let result = result.ok_or_else(|| AppError::usage("--observe 需要 --result"))?;
    observe_result(&observe, &result, emotion_after, json)
}

// ---------- 写：记一条 ----------

#[derive(Serialize)]
struct Recurring {
    label: String,
    count: usize,
}

#[derive(Serialize)]
struct RecordOutcome {
    id: String,
    at: String,
    missing: Vec<&'static str>,
    next_question: &'static str,
    recurring: Vec<Recurring>,
}

fn record(json: bool) -> Result<(), AppError> {
    let mut text = String::new();
    io::stdin()
        .read_to_string(&mut text)
        .map_err(|err| AppError::usage(format!("读取标准输入失败：{err}")))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|err| AppError::usage(format!("标准输入不是合法 JSON：{err}")))?;
    let mut reflection = parse_input(&value)?;

    let mut ledger = load()?;
    let id = new_id("ref", |candidate| {
        ledger.reflections.iter().any(|item| item.id == candidate)
    });
    reflection.id = id.clone();
    reflection.at = now_iso8601();
    ledger.reflections.push(reflection.clone());
    save(&ledger)?;

    let missing = missing_fields(&reflection);
    let outcome = RecordOutcome {
        id: reflection.id,
        at: reflection.at,
        missing: missing.clone(),
        next_question: next_question(&missing),
        recurring: recurring(&ledger),
    };
    if json {
        println!("{}", to_pretty(&outcome)?);
    } else {
        println!("已记 {}", outcome.id);
        println!("还差：{}", summarize_missing(&missing));
        println!("下一问：{}", outcome.next_question);
        print_recurring(&outcome.recurring);
    }
    Ok(())
}

// ---------- 读：一条与总表 ----------

#[derive(Serialize)]
struct One {
    #[serde(flatten)]
    reflection: Reflection,
    missing: Vec<&'static str>,
}

fn print_one(id: &str, json: bool) -> Result<(), AppError> {
    let ledger = load()?;
    let reflection = find(&ledger, id)?;
    let missing = missing_fields(reflection);
    if json {
        println!(
            "{}",
            to_pretty(&One {
                reflection: reflection.clone(),
                missing,
            })?
        );
        return Ok(());
    }
    println!("{}  {}", reflection.id, reflection.at);
    println!("情境：{}", reflection.situation);
    println!("自动思维：{}", reflection.thought);
    if let Some(emotion) = &reflection.emotion {
        println!("情绪：{} {}", emotion.label, emotion.intensity);
    }
    if !reflection.distortions.is_empty() {
        println!("认知扭曲：{}", reflection.distortions.join("、"));
    }
    if !reflection.evidence_for.is_empty() {
        println!("支持证据：{}", reflection.evidence_for.join("；"));
    }
    if !reflection.evidence_against.is_empty() {
        println!("反对证据：{}", reflection.evidence_against.join("；"));
    }
    if let Some(balanced) = &reflection.balanced {
        println!("替代思维：{balanced}");
    }
    if let Some(experiment) = &reflection.experiment {
        println!("实验预测：{}", experiment.prediction);
        if let Some(review_on) = &experiment.review_on {
            println!("复看日：{review_on}");
        }
        if let Some(result) = &experiment.result {
            println!("结果：{result}");
        }
    }
    println!("还差：{}", summarize_missing(&missing));
    Ok(())
}

#[derive(Serialize)]
struct DueItem {
    id: String,
    situation: String,
    review_on: String,
    prediction: String,
}

#[derive(Serialize)]
struct OpenItem {
    id: String,
    situation: String,
    missing: Vec<&'static str>,
}

#[derive(Serialize)]
struct ListOutcome {
    due: Vec<DueItem>,
    open: Vec<OpenItem>,
    recurring: Vec<Recurring>,
}

fn print_list(due_only: bool, json: bool) -> Result<(), AppError> {
    let ledger = load()?;
    let today = today();
    let mut due = Vec::new();
    let mut open = Vec::new();
    for reflection in &ledger.reflections {
        if let Some(experiment) = &reflection.experiment
            && experiment.result.is_none()
            && let Some(review_on) = &experiment.review_on
            && *review_on <= today
        {
            due.push(DueItem {
                id: reflection.id.clone(),
                situation: reflection.situation.clone(),
                review_on: review_on.clone(),
                prediction: experiment.prediction.clone(),
            });
        }
        let missing = missing_fields(reflection);
        if !due_only && experiment_result_is_none(reflection) && !missing.is_empty() {
            open.push(OpenItem {
                id: reflection.id.clone(),
                situation: reflection.situation.clone(),
                missing,
            });
        }
    }
    let outcome = ListOutcome {
        due,
        open,
        recurring: recurring(&ledger),
    };
    if json {
        println!("{}", to_pretty(&outcome)?);
        return Ok(());
    }
    if outcome.due.is_empty() {
        println!("没有到期待复看的实验");
    } else {
        println!("到期复看 {} 条", outcome.due.len());
        for item in &outcome.due {
            println!(
                "  {} {}  预测：{}",
                item.id, item.situation, item.prediction
            );
        }
    }
    if !due_only {
        if outcome.open.is_empty() {
            println!("没有半截的记录");
        } else {
            let shown = OPEN_SHOWN.min(outcome.open.len());
            if outcome.open.len() > OPEN_SHOWN {
                println!("未填完 {} 条（只列最近 {shown} 条）", outcome.open.len());
            } else {
                println!("未填完 {} 条", outcome.open.len());
            }
            for item in outcome.open.iter().rev().take(OPEN_SHOWN) {
                println!(
                    "  {} {}  还差：{}",
                    item.id,
                    item.situation,
                    summarize_missing(&item.missing)
                );
            }
            if outcome.open.len() > OPEN_SHOWN {
                println!(
                    "另有 {} 条未列出，用 --json 看全部",
                    outcome.open.len() - OPEN_SHOWN
                );
            }
        }
        print_recurring(&outcome.recurring);
    }
    Ok(())
}

// ---------- 回填：行为实验的结果 ----------

#[derive(Serialize)]
struct ObserveOutcome {
    id: String,
    prediction: String,
    result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    emotion_after: Option<u8>,
}

fn observe_result(
    id: &str,
    result: &str,
    emotion_after: Option<u8>,
    json: bool,
) -> Result<(), AppError> {
    if result.trim().is_empty() {
        return Err(AppError::usage("--result 不能为空"));
    }
    if let Some(value) = emotion_after
        && value > 100
    {
        return Err(AppError::usage("--emotion 是 0-100 的强度"));
    }
    let mut ledger = load()?;
    let position = ledger
        .reflections
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| AppError::NotFound {
            message: format!("找不到反思 {id}"),
            hint: "用 --list 看有哪些未回填的实验".to_string(),
        })?;
    let experiment = ledger.reflections[position]
        .experiment
        .as_mut()
        .ok_or_else(|| AppError::usage(format!("{id} 没有行为实验，无从回填")))?;
    experiment.result = Some(result.to_string());
    experiment.emotion_after = emotion_after;
    let prediction = experiment.prediction.clone();
    save(&ledger)?;

    let outcome = ObserveOutcome {
        id: id.to_string(),
        prediction,
        result: result.to_string(),
        emotion_after,
    };
    if json {
        println!("{}", to_pretty(&outcome)?);
    } else {
        println!("已回填 {id}：{result}");
        println!("预测：{}", outcome.prediction);
    }
    Ok(())
}

// ---------- 解析与校验 ----------

fn parse_input(value: &Value) -> Result<Reflection, AppError> {
    let situation = required_text(value, "situation")?;
    let thought = required_text(value, "thought")?;

    let emotion = parse_emotion(value)?;
    let distortions = parse_distortions(value)?;
    let evidence_for = text_list(value, "evidence_for")?;
    let evidence_against = text_list(value, "evidence_against")?;
    let balanced = optional_text(value, "balanced")?;
    let domain = parse_enum(value, "domain", DOMAINS)?;
    let control = parse_enum(value, "control", CONTROLS)?;
    let experiment = parse_experiment(value)?;

    Ok(Reflection {
        id: String::new(),
        at: String::new(),
        situation,
        thought,
        emotion,
        distortions,
        evidence_for,
        evidence_against,
        balanced,
        domain,
        control,
        experiment,
    })
}

fn parse_emotion(value: &Value) -> Result<Option<Emotion>, AppError> {
    let label = optional_text(value, "emotion")?;
    let intensity = value.get("intensity").and_then(Value::as_u64);
    match (label, intensity) {
        (None, None) => Ok(None),
        (Some(label), Some(intensity)) => {
            if intensity > 100 {
                return Err(AppError::usage("intensity 是 0-100 的强度"));
            }
            Ok(Some(Emotion {
                label,
                intensity: intensity as u8,
            }))
        }
        (Some(_), None) => Err(AppError::usage("给了 emotion 就要给 intensity（0-100）")),
        (None, Some(_)) => Err(AppError::usage("给了 intensity 就要给 emotion")),
    }
}

fn parse_distortions(value: &Value) -> Result<Vec<String>, AppError> {
    let mut out = Vec::new();
    for item in text_list(value, "distortions")? {
        out.push(normalize_distortion(&item)?);
    }
    Ok(out)
}

fn parse_enum(value: &Value, field: &str, allowed: &[&str]) -> Result<Option<String>, AppError> {
    let Some(text) = optional_text(value, field)? else {
        return Ok(None);
    };
    if allowed.contains(&text.as_str()) {
        return Ok(Some(text));
    }
    Err(AppError::usage(format!(
        "{field} 只接受 {}，收到「{text}」",
        allowed.join("／")
    )))
}

fn parse_experiment(value: &Value) -> Result<Option<Experiment>, AppError> {
    let Some(experiment) = value.get("experiment") else {
        return Ok(None);
    };
    if experiment.is_null() {
        return Ok(None);
    }
    let prediction = required_text(experiment, "prediction")?;
    let review_on = optional_text(experiment, "review_on")?;
    if let Some(date) = &review_on
        && !is_date(date)
    {
        return Err(AppError::usage(format!(
            "review_on 要写 YYYY-MM-DD，收到「{date}」"
        )));
    }
    Ok(Some(Experiment {
        prediction,
        review_on,
        result: None,
        emotion_after: None,
    }))
}

fn normalize_distortion(raw: &str) -> Result<String, AppError> {
    let key = raw.trim();
    for (english, chinese) in DISTORTIONS {
        if key.eq_ignore_ascii_case(english) || key == *chinese {
            return Ok((*chinese).to_string());
        }
    }
    let known: Vec<&str> = DISTORTIONS.iter().map(|(_, chinese)| *chinese).collect();
    Err(AppError::usage(format!(
        "不认识的认知扭曲「{key}」，可用：{}",
        known.join("／")
    )))
}

fn required_text(value: &Value, field: &str) -> Result<String, AppError> {
    let text = value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if text.is_empty() {
        return Err(AppError::usage(format!("缺字段 {field}（不能为空）")));
    }
    Ok(text.to_string())
}

fn optional_text(value: &Value, field: &str) -> Result<Option<String>, AppError> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if text.trim().is_empty() => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.trim().to_string())),
        Some(_) => Err(AppError::usage(format!("{field} 要写字符串"))),
    }
}

fn text_list(value: &Value, field: &str) -> Result<Vec<String>, AppError> {
    let Some(items) = value.get(field) else {
        return Ok(Vec::new());
    };
    if items.is_null() {
        return Ok(Vec::new());
    }
    let array = items
        .as_array()
        .ok_or_else(|| AppError::usage(format!("{field} 要写字符串数组")))?;
    let mut out = Vec::new();
    for item in array {
        let text = item
            .as_str()
            .ok_or_else(|| AppError::usage(format!("{field} 里的每一项都要是字符串")))?
            .trim();
        if !text.is_empty() {
            out.push(text.to_string());
        }
    }
    Ok(out)
}

fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

// ---------- 汇总 ----------

fn missing_fields(reflection: &Reflection) -> Vec<&'static str> {
    let mut out = Vec::new();
    if reflection.emotion.is_none() {
        out.push("emotion");
    }
    if reflection.distortions.is_empty() {
        out.push("distortions");
    }
    if reflection.evidence_against.is_empty() {
        out.push("evidence_against");
    }
    if reflection.balanced.is_none() {
        out.push("balanced");
    }
    if reflection.experiment.is_none() {
        out.push("experiment");
    }
    out
}

fn next_question(missing: &[&str]) -> &'static str {
    let Some(first) = missing.first() else {
        return COMPLETE_QUESTION;
    };
    QUESTIONS
        .iter()
        .find(|(field, _)| field == first)
        .map(|(_, question)| *question)
        .unwrap_or(COMPLETE_QUESTION)
}

fn summarize_missing(missing: &[&str]) -> String {
    if missing.is_empty() {
        return "无，记录完整".to_string();
    }
    let names: Vec<&str> = missing
        .iter()
        .map(|field| match *field {
            "emotion" => "情绪与强度",
            "distortions" => "认知扭曲",
            "evidence_against" => "反对证据",
            "balanced" => "替代思维",
            "experiment" => "行为实验",
            other => other,
        })
        .collect();
    names.join("、")
}

fn recurring(ledger: &Ledger) -> Vec<Recurring> {
    let mut groups: Vec<Recurring> = Vec::new();
    for reflection in &ledger.reflections {
        for label in &reflection.distortions {
            match groups.iter_mut().find(|group| group.label == *label) {
                Some(group) => group.count += 1,
                None => groups.push(Recurring {
                    label: label.clone(),
                    count: 1,
                }),
            }
        }
    }
    groups.retain(|group| group.count >= 2);
    groups.sort_by_key(|group| std::cmp::Reverse(group.count));
    groups
}

fn print_recurring(groups: &[Recurring]) {
    if groups.is_empty() {
        return;
    }
    let text: Vec<String> = groups
        .iter()
        .map(|group| format!("{} {}", group.label, group.count))
        .collect();
    println!("反复出现：{}", text.join("、"));
}

/// 有实验但没回填结果的记录，用来判断「半截」。
fn experiment_result_is_none(reflection: &Reflection) -> bool {
    match &reflection.experiment {
        Some(experiment) => experiment.result.is_none(),
        None => true,
    }
}

fn find<'a>(ledger: &'a Ledger, id: &str) -> Result<&'a Reflection, AppError> {
    ledger
        .reflections
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| AppError::NotFound {
            message: format!("找不到反思 {id}"),
            hint: "用 --list 看有哪些记录".to_string(),
        })
}

// ---------- 存储 ----------

fn data_file() -> PathBuf {
    data_dir().join(DATA_FILE)
}

fn load() -> Result<Ledger, AppError> {
    let path = data_file();
    match fs::read_to_string(&path) {
        Ok(text) if text.trim().is_empty() => Ok(Ledger::default()),
        Ok(text) => serde_json::from_str(&text)
            .map_err(|err| AppError::usage(format!("数据文件解析失败 {}：{err}", path.display()))),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Ledger::default()),
        Err(err) => Err(AppError::usage(format!(
            "无法读取 {}：{err}",
            path.display()
        ))),
    }
}

fn save(ledger: &Ledger) -> Result<(), AppError> {
    let path = data_file();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|err| AppError::usage(format!("无法创建目录 {}：{err}", parent.display())))?;
    }
    let mut text = serde_json::to_string_pretty(ledger)
        .map_err(|err| AppError::usage(format!("数据序列化失败：{err}")))?;
    text.push('\n');
    fs::write(&path, text)
        .map_err(|err| AppError::usage(format!("无法写入 {}：{err}", path.display())))
}

/// 今天（UTC，YYYY-MM-DD），与 `at` 同一时区口径。
fn today() -> String {
    now_iso8601().chars().take(10).collect()
}

fn to_pretty<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string_pretty(value)
        .map_err(|err| AppError::usage(format!("JSON 序列化失败：{err}")))
}
