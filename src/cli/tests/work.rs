//! `work intent` / `work clarify` 集成测试：每条命令、每个 flag、每条错误路径。
//!
//! 数据目录一律指向 tempfile（`QTFOUNDER_LAB_DATA`），不污染仓库 `data/`。

use std::fs;
use std::process::Output;

use assert_cmd::Command;
use tempfile::TempDir;

/// 临时数据目录（整个数据目录，含 clarify.json 与 rules.md）。
fn lab() -> TempDir {
    TempDir::new().expect("临时目录")
}

/// 取一个绑定到该数据目录的命令（assert_cmd 的 Command 不可克隆，每次新建）。
fn cmd(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("qtfounder-lab").expect("构建二进制");
    cmd.env("QTFOUNDER_LAB_DATA", dir.path());
    cmd
}

fn stdin_json(task: Option<&str>, before: &str, actual: &str, prompt: &str) -> String {
    let task = match task {
        Some(task) => format!("\"task\":\"{task}\","),
        None => String::new(),
    };
    format!(r#"{{{task}"before":"{before}","actual":"{actual}","prompt":"{prompt}"}}"#)
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn store_json(dir: &TempDir) -> serde_json::Value {
    let text = fs::read_to_string(dir.path().join("clarify.json")).expect("clarify.json 应已写入");
    serde_json::from_str(&text).expect("clarify.json 应是合法 JSON")
}

/// 在原始 JSON 文本里按顺序断言字段出现（serde_json 的 Value 解析会打乱顺序，只能看原文）。
fn assert_key_order(json: &str, keys: &[&str]) {
    let mut cursor = 0;
    for key in keys {
        let needle = format!("\"{key}\"");
        let pos = json[cursor..]
            .find(&needle)
            .unwrap_or_else(|| panic!("{json} 中 {cursor} 之后缺字段 {key}"));
        cursor += pos + needle.len();
    }
}

/// 用 --stdin 立一件事 + 一条澄清，返回 intent id。
fn seed_intent(dir: &TempDir, task: &str) -> String {
    let out = cmd(dir)
        .args(["work", "clarify", "--stdin", "--json"])
        .write_stdin(stdin_json(
            Some(task),
            "它知道我只要三行结论",
            "铺开写了十段解释",
            "先给三行以内的结论",
        ))
        .output()
        .expect("执行 clarify");
    assert!(out.status.success(), "seed 失败：{}", stderr_of(&out));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).expect("seed 输出 JSON");
    value["intent_id"].as_str().expect("intent_id").to_string()
}

// ---------- intent：列表 ----------

#[test]
fn intent_list_json_empty() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "intent", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(stdout_of(&out), "{\n  \"intents\": []\n}\n");
    assert!(stderr_of(&out).is_empty());
}

#[test]
fn intent_list_human_empty() {
    let dir = lab();
    let out = cmd(&dir).args(["work", "intent"]).output().unwrap();
    assert!(out.status.success());
    assert!(stdout_of(&out).contains("暂无意图"));
}

#[test]
fn intent_list_json_counts_and_candidates() {
    let dir = lab();
    // 同一件事里：两条归一化后相同的差 + 一条不同的差
    let task = "给这段代码写单元测试";
    cmd(&dir)
        .args(["work", "clarify", "--stdin", "--json"])
        .write_stdin(stdin_json(
            Some(task),
            "它知道项目用 pytest",
            "Gave five paragraphs, not code!",
            "需要能直接跑的代码",
        ))
        .assert()
        .success();
    cmd(&dir)
        .args(["work", "clarify", "--stdin", "--json"])
        .write_stdin(stdin_json(
            None,
            "它知道 fixture 在 conftest",
            "gave five paragraphs not code",
            "需要能直接跑的代码",
        ))
        .assert()
        .success();
    cmd(&dir)
        .args(["work", "clarify", "--stdin", "--json"])
        .write_stdin(stdin_json(
            None,
            "它知道我要先看结论",
            "答非所问",
            "回到问题本身",
        ))
        .assert()
        .success();

    let out = cmd(&dir)
        .args(["work", "intent", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let intents = value["intents"].as_array().unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0]["task"], task);
    assert_eq!(intents[0]["clarifications"], 3);
    // 字段顺序与契约一致：id / task / at / clarifications / candidates
    assert_key_order(
        &stdout_of(&out),
        &["id", "task", "at", "clarifications", "candidates"],
    );

    let candidates = intents[0]["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 1);
    // 归一化：小写、去标点、去空白后同类归一
    assert_eq!(candidates[0]["actual_key"], "gavefiveparagraphsnotcode");
    assert_eq!(candidates[0]["count"], 2);
    let ids = candidates[0]["clarification_ids"].as_array().unwrap();
    assert_eq!(ids.len(), 2);
    assert!(
        ids.iter()
            .all(|id| id.as_str().unwrap().starts_with("clr_"))
    );
}

#[test]
fn intent_list_human_with_candidates() {
    let dir = lab();
    let task = "把日志里的观察站段落切出来";
    for (i, before) in ["它知道只要切段不要解释", "它知道不要附带展开说明"]
        .iter()
        .enumerate()
    {
        cmd(&dir)
            .args(["work", "clarify", "--stdin"])
            .write_stdin(stdin_json(
                if i == 0 { Some(task) } else { None },
                before,
                "整段照搬没有切分",
                "按段落切开重发",
            ))
            .assert()
            .success();
    }
    let out = cmd(&dir).args(["work", "intent"]).output().unwrap();
    assert!(out.status.success());
    let text = stdout_of(&out);
    assert!(text.contains(task), "输出应含任务：{text}");
    assert!(text.contains("2 次澄清"), "输出应含澄清次数：{text}");
    assert!(
        text.contains("候选：整段照搬没有切分"),
        "输出应含候选：{text}"
    );
    assert!(text.contains("×2"), "输出应含重复次数：{text}");
}

// ---------- intent：详情 ----------

#[test]
fn intent_detail_json() {
    let dir = lab();
    let task = "给这段代码写单元测试";
    let intent_id = seed_intent(&dir, task);

    let out = cmd(&dir)
        .args(["work", "intent", "--id", &intent_id, "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["id"], intent_id);
    assert_eq!(value["task"], task);
    let at = value["at"].as_str().unwrap();
    assert!(
        at.ends_with('Z') && at.len() == 20,
        "at 应是 ISO 8601：{at}"
    );
    let cls = value["clarifications"].as_array().unwrap();
    assert_eq!(cls.len(), 1);
    // 详情里澄清记录的五个字段一个不少
    for field in ["id", "at", "before", "actual", "prompt"] {
        assert!(!cls[0][field].is_null(), "详情缺字段 {field}");
    }
    assert_eq!(cls[0]["before"], "它知道我只要三行结论");
    assert_eq!(cls[0]["actual"], "铺开写了十段解释");
    assert_eq!(cls[0]["prompt"], "先给三行以内的结论");
}

#[test]
fn intent_detail_human() {
    let dir = lab();
    let intent_id = seed_intent(&dir, "给这段代码写单元测试");
    let out = cmd(&dir)
        .args(["work", "intent", "--id", &intent_id])
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = stdout_of(&out);
    assert!(text.contains(&intent_id));
    assert!(text.contains("原以为：它知道我只要三行结论"));
    assert!(text.contains("实际：铺开写了十段解释"));
    assert!(text.contains("提示词：先给三行以内的结论"));
}

// ---------- intent：错误 ----------

#[test]
fn intent_id_not_found_exits_3() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "intent", "--id", "int_ZZZZ", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(stdout_of(&out).is_empty());
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).expect("错误应是 JSON");
    assert_eq!(err["error"]["code"], "NOT_FOUND");
    assert_eq!(err["error"]["message"], "int_ZZZZ 不存在");
    assert_eq!(err["error"]["hint"], "用 qtfounder-lab work intent 查看");
}

#[test]
fn intent_unknown_flag_exits_2() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "intent", "--foo"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "USAGE");
    assert_eq!(err["error"]["message"], "未知参数 --foo");
    assert!(err["error"].get("hint").is_none(), "USAGE 不带 hint");
}

#[test]
fn intent_id_without_value_exits_2() {
    let dir = lab();
    let out = cmd(&dir).args(["work", "intent", "--id"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "USAGE");
    assert!(err["error"]["message"].as_str().unwrap().contains("--id"));
}

// ---------- clarify：交互 ----------

#[test]
fn clarify_interactive_opens_first_task() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "clarify"])
        .write_stdin(
            "给这段代码写单元测试\n它知道项目用 pytest，fixture 在 conftest\n给了五段解释加伪代码\n",
        )
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr_of(&out));
    let text = stdout_of(&out);
    assert!(text.contains("任务："), "新开时应问任务：{text}");
    assert!(text.contains("你原本以为它知道 / 会做什么？"));
    assert!(text.contains("它实际做了什么？"));
    assert!(text.contains("我原以为「它知道项目用 pytest，fixture 在 conftest」"));
    assert!(text.contains("已记入 int_"));

    let store = store_json(&dir);
    let intents = store["intents"].as_array().unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0]["task"], "给这段代码写单元测试");
    assert_eq!(intents[0]["clarifications"].as_array().unwrap().len(), 1);
}

#[test]
fn clarify_interactive_attaches_recent_without_task_prompt() {
    let dir = lab();
    let intent_id = seed_intent(&dir, "给这段代码写单元测试");

    let out = cmd(&dir)
        .args(["work", "clarify"])
        .write_stdin("它知道我要能直接跑的代码\n这次给了三段解释加伪代码\n")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr_of(&out));
    let text = stdout_of(&out);
    assert!(!text.contains("任务："), "挂最近一件时不该问任务：{text}");
    assert!(text.contains(&format!("已记入 {intent_id} / clr_")));
}

#[test]
fn clarify_interactive_reasks_vague_answer() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "clarify"])
        .write_stdin(
            "第一件事先立起来\n它就是不行\n它把表格贴成了截图，我要可复制的文本\n给了整段客套话\n",
        )
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr_of(&out));
    let text = stdout_of(&out);
    assert!(
        text.contains("举它刚才那句话或那个动作？"),
        "答得笼统应追问一次：{text}"
    );
    assert!(text.contains("已记入"));
}

#[test]
fn clarify_interactive_eof_exits_2() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "clarify"])
        .write_stdin("")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "USAGE");
    assert_eq!(err["error"]["message"], "缺少回答");
}

// ---------- clarify：--task ----------

#[test]
fn clarify_task_opens_new_intent() {
    let dir = lab();
    seed_intent(&dir, "给这段代码写单元测试");

    let out = cmd(&dir)
        .args(["work", "clarify", "--task", "把日志里的观察站段落切出来"])
        .write_stdin("它知道只切段不要解释\n整段照搬没有切分\n按段落切开重发\n")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr_of(&out));
    let text = stdout_of(&out);
    assert!(
        !text.contains("任务："),
        "--task 已给就不该再问任务：{text}"
    );

    let store = store_json(&dir);
    let intents = store["intents"].as_array().unwrap();
    assert_eq!(intents.len(), 2, "应新开一件事");
    assert_eq!(intents[1]["task"], "把日志里的观察站段落切出来");
    assert_eq!(intents[1]["clarifications"].as_array().unwrap().len(), 1);
    assert_eq!(intents[0]["clarifications"].as_array().unwrap().len(), 1);
}

#[test]
fn clarify_task_json_flag_wins_over_stdin_task() {
    let dir = lab();
    let out = cmd(&dir)
        .args([
            "work",
            "clarify",
            "--task",
            "flag 开的事",
            "--stdin",
            "--json",
        ])
        .write_stdin(stdin_json(
            Some("stdin 里的事"),
            "它知道我要结论",
            "写了三页铺陈",
            "先给结论",
        ))
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr_of(&out));
    let store = store_json(&dir);
    assert_eq!(store["intents"][0]["task"], "flag 开的事");
}

// ---------- clarify：--stdin ----------

#[test]
fn clarify_stdin_json_full() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "clarify", "--stdin", "--json"])
        .write_stdin(stdin_json(
            Some("给这段代码写单元测试"),
            "它知道项目用 pytest、fixture 在 conftest",
            "给了五段解释加伪代码",
            "刚才你的回答偏在铺陈展开，我需要的是能直接跑的代码",
        ))
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr_of(&out));

    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    // 五字段：id / intent_id / at / deduplicated / prompt
    assert_key_order(
        &stdout_of(&out),
        &["id", "intent_id", "at", "deduplicated", "prompt"],
    );
    for field in ["id", "intent_id", "at", "deduplicated", "prompt"] {
        assert!(!value[field].is_null(), "输出缺字段 {field}");
    }
    assert!(value["id"].as_str().unwrap().starts_with("clr_"));
    assert!(value["intent_id"].as_str().unwrap().starts_with("int_"));
    assert_eq!(value["deduplicated"], false);
    assert_eq!(
        value["prompt"],
        "刚才你的回答偏在铺陈展开，我需要的是能直接跑的代码"
    );

    let store = store_json(&dir);
    assert_eq!(store["intents"].as_array().unwrap().len(), 1);
}

#[test]
fn clarify_stdin_attaches_without_task() {
    let dir = lab();
    let intent_id = seed_intent(&dir, "给这段代码写单元测试");

    let out = cmd(&dir)
        .args(["work", "clarify", "--stdin", "--json"])
        .write_stdin(stdin_json(
            None,
            "它知道我要结论",
            "写了三页铺陈",
            "先给结论",
        ))
        .output()
        .unwrap();
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["intent_id"], intent_id, "应挂到最近一件事上");
    assert_eq!(value["deduplicated"], false);
}

#[test]
fn clarify_stdin_missing_field_exits_2() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "clarify", "--stdin", "--json"])
        .write_stdin(r#"{"task":"x","before":"它知道我要结论","prompt":"先给结论"}"#)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(stdout_of(&out).is_empty());
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "USAGE");
    assert_eq!(err["error"]["message"], "缺少字段 actual");
}

#[test]
fn clarify_stdin_empty_text_field_exits_2() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "clarify", "--stdin"])
        .write_stdin(stdin_json(Some("x"), "它知道我要结论", "   ", "先给结论"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["message"], "缺少字段 actual");
}

#[test]
fn clarify_stdin_bad_json_exits_2() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "clarify", "--stdin"])
        .write_stdin(r#"{"before":"它知道我要结论","actual":"#)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "USAGE");
    assert!(
        err["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("JSON 解析失败：第 1 行"),
        "实际：{}",
        err["error"]["message"]
    );
}

#[test]
fn clarify_stdin_wrong_type_exits_2() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "clarify", "--stdin"])
        .write_stdin(r#"{"before":"它知道我要结论","actual":42,"prompt":"先给结论"}"#)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["message"], "字段 actual 必须是文本");
}

#[test]
fn clarify_stdin_without_task_and_no_intent_exits_2() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "clarify", "--stdin"])
        .write_stdin(stdin_json(
            None,
            "它知道我要结论",
            "写了三页铺陈",
            "先给结论",
        ))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "USAGE");
    assert_eq!(err["error"]["message"], "没有可挂的意图，缺少 task");
}

// ---------- clarify：幂等去重 ----------

#[test]
fn clarify_dedup_is_idempotent() {
    let dir = lab();
    let payload = stdin_json(
        Some("给这段代码写单元测试"),
        "它知道项目用 pytest、fixture 在 conftest",
        "给了五段解释加伪代码",
        "需要能直接跑的代码",
    );

    let first = cmd(&dir)
        .args(["work", "clarify", "--stdin", "--json"])
        .write_stdin(payload.clone())
        .output()
        .unwrap();
    assert!(first.status.success());
    let first_value: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(first_value["deduplicated"], false);

    // 同一条重复提交：不新增，deduplicated=true，退出码 0
    let second = cmd(&dir)
        .args(["work", "clarify", "--stdin", "--json"])
        .write_stdin(payload)
        .output()
        .unwrap();
    assert_eq!(second.status.code(), Some(0), "去重退出码仍是 0");
    let second_value: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(second_value["deduplicated"], true);
    assert_eq!(second_value["id"], first_value["id"]);
    assert_eq!(second_value["intent_id"], first_value["intent_id"]);
    assert_eq!(second_value["prompt"], first_value["prompt"]);

    let store = store_json(&dir);
    assert_eq!(store["intents"].as_array().unwrap().len(), 1);
    assert_eq!(
        store["intents"][0]["clarifications"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn clarify_dedup_human_output() {
    let dir = lab();
    let payload = stdin_json(
        Some("第一件事"),
        "它知道我要结论",
        "写了三页铺陈",
        "先给结论",
    );
    cmd(&dir)
        .args(["work", "clarify", "--stdin"])
        .write_stdin(payload.clone())
        .assert()
        .success();
    let out = cmd(&dir)
        .args(["work", "clarify", "--stdin"])
        .write_stdin(payload)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let text = stdout_of(&out);
    assert!(text.contains("已存在 int_"), "重复提交人读输出：{text}");
    assert!(text.contains("重复提交，未新增记录"));
}

// ---------- clarify：--rule ----------

/// 造一个重复 ≥2 次的候选：两条 actual 相同、before 不同（避免去重）。
fn seed_duplicate(dir: &TempDir) {
    for (i, before) in ["它知道我要结论", "它知道要三行以内"].iter().enumerate() {
        cmd(dir)
            .args(["work", "clarify", "--stdin"])
            .write_stdin(stdin_json(
                if i == 0 { Some("第一件事") } else { None },
                before,
                "铺开写了十段解释",
                "先给三行结论",
            ))
            .assert()
            .success();
    }
}

#[test]
fn clarify_rule_appends_human() {
    let dir = lab();
    seed_duplicate(&dir);

    let out = cmd(&dir)
        .args([
            "work",
            "clarify",
            "--rule",
            "先结论后细节，除非我明确要展开",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr_of(&out));
    let text = stdout_of(&out);
    let rules_path = dir.path().join("rules.md");
    assert!(text.contains(&format!("已追加规则到 {}", rules_path.display())));
    assert!(
        text.contains("- 先结论后细节，除非我明确要展开"),
        "人读输出应含规则行：{text}"
    );
    assert!(text.contains("已处理 2 条重复差"), "应报处理的差：{text}");

    let rules = fs::read_to_string(&rules_path).expect("rules.md 应写入");
    assert_eq!(rules, "- 先结论后细节，除非我明确要展开\n");

    // 重复的差标为已处理
    let store = store_json(&dir);
    let processed = store["processed_clarifications"].as_array().unwrap();
    assert_eq!(processed.len(), 2);
}

#[test]
fn clarify_rule_json_and_source_only_unprocessed() {
    let dir = lab();
    seed_duplicate(&dir);

    let first = cmd(&dir)
        .args(["work", "clarify", "--rule", "先结论后细节", "--json"])
        .output()
        .unwrap();
    assert!(first.status.success());
    let value: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(value["rule"], "先结论后细节");
    let source = value["source"].as_array().unwrap();
    assert_eq!(source.len(), 2, "首次立规则应给出全部未处理的重复差");
    assert!(
        source
            .iter()
            .all(|id| id.as_str().unwrap().starts_with("clr_"))
    );
    assert!(
        value["path"].as_str().unwrap().ends_with("rules.md"),
        "path 应指向 rules.md：{}",
        value["path"]
    );

    // 第二次立规则：重复差已处理，source 为空
    let second = cmd(&dir)
        .args(["work", "clarify", "--rule", "追问再展开", "--json"])
        .output()
        .unwrap();
    assert!(second.status.success());
    let value: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(value["source"].as_array().unwrap().len(), 0);

    let rules = fs::read_to_string(dir.path().join("rules.md")).unwrap();
    assert_eq!(rules, "- 先结论后细节\n- 追问再展开\n");
    let store = store_json(&dir);
    assert_eq!(
        store["processed_clarifications"].as_array().unwrap().len(),
        2
    );
}

#[test]
fn clarify_task_and_rule_mutex_exits_2() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "clarify", "--task", "x", "--rule", "y"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "USAGE");
    assert_eq!(err["error"]["message"], "--task 与 --rule 不能同时使用");
}

#[test]
fn clarify_rule_empty_text_exits_2() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["work", "clarify", "--rule", ""])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "USAGE");
    assert_eq!(err["error"]["message"], "--rule 需要一段文本");
}

// ---------- 通用：数据目录、退出码、用法 ----------

#[test]
fn clarify_default_data_dir_under_cwd() {
    let dir = TempDir::new().unwrap();
    let out = Command::cargo_bin("qtfounder-lab")
        .unwrap()
        .current_dir(dir.path())
        .args(["work", "clarify", "--stdin", "--json"])
        .write_stdin(stdin_json(
            Some("默认数据目录的事"),
            "它知道我要结论",
            "写了三页铺陈",
            "先给结论",
        ))
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(
        dir.path().join("data").join("clarify.json").is_file(),
        "不给 QTFOUNDER_LAB_DATA 时应落 ./data/clarify.json"
    );
}

#[test]
fn clarify_data_dir_io_error_exits_2() {
    let dir = TempDir::new().unwrap();
    // 数据目录位置放一个普通文件，读写都会失败
    let blocked = dir.path().join("blocked");
    fs::write(&blocked, "not a directory").unwrap();
    let out = Command::cargo_bin("qtfounder-lab")
        .unwrap()
        .env("QTFOUNDER_LAB_DATA", &blocked)
        .args(["work", "clarify", "--stdin"])
        .write_stdin(stdin_json(
            Some("x"),
            "它知道我要结论",
            "写了三页铺陈",
            "先给结论",
        ))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "USAGE");
}

#[test]
fn work_without_subcommand_exits_2() {
    let dir = lab();
    let out = cmd(&dir).arg("work").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "USAGE");
}

#[test]
fn help_exits_zero() {
    let dir = lab();
    let out = cmd(&dir).arg("--help").output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout_of(&out).contains("work"));

    // 子命令帮助里列全两个入口
    let out = cmd(&dir).args(["work", "--help"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let text = stdout_of(&out);
    assert!(text.contains("intent"), "帮助应列 intent：{text}");
    assert!(text.contains("clarify"), "帮助应列 clarify：{text}");
}
