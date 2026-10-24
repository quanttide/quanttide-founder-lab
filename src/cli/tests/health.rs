//! `health reflect` 集成测试：记一条、看总表、回填实验、错误路径。
//!
//! 数据目录一律指向 tempfile（`QTFOUNDER_LAB_DATA`），不污染仓库 `data/`。

use std::fs;
use std::process::Output;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn lab() -> TempDir {
    TempDir::new().expect("临时目录")
}

fn cmd(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("qtfounder-lab").expect("构建二进制");
    cmd.env("QTFOUNDER_LAB_DATA", dir.path());
    cmd
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// 走一次 `--stdin --json`，断言成功并返回解析后的结果。
fn record(dir: &TempDir, body: &str) -> Value {
    let out = cmd(dir)
        .args(["health", "reflect", "--stdin", "--json"])
        .write_stdin(body)
        .output()
        .expect("运行二进制");
    assert!(out.status.success(), "记录应成功：{}", stderr_of(&out));
    serde_json::from_str(&stdout_of(&out)).expect("标准输出应是 JSON")
}

fn record_fails(dir: &TempDir, body: &str) -> Output {
    cmd(dir)
        .args(["health", "reflect", "--stdin", "--json"])
        .write_stdin(body)
        .output()
        .expect("运行二进制")
}

fn ledger(dir: &TempDir) -> Value {
    let text = fs::read_to_string(dir.path().join("reflect.json")).expect("reflect.json 应已写入");
    serde_json::from_str(&text).expect("reflect.json 应是合法 JSON")
}

fn list(dir: &TempDir, extra: &[&str]) -> Value {
    let mut args = vec!["health", "reflect", "--list", "--json"];
    args.extend_from_slice(extra);
    let out = cmd(dir).args(&args).output().expect("运行二进制");
    assert!(out.status.success(), "列出应成功：{}", stderr_of(&out));
    serde_json::from_str(&stdout_of(&out)).expect("标准输出应是 JSON")
}

const FULL: &str = r#"{
  "situation": "周三亲自去谈供应商",
  "thought": "只有我去谈才不会谈砸",
  "emotion": "焦虑",
  "intensity": 70,
  "distortions": ["personalization", "catastrophizing"],
  "evidence_for": ["上次对方报价高"],
  "evidence_against": ["团队谈过三次，只差一个点"],
  "balanced": "我去能多拿一点，但团队够用；我去是因为不放心",
  "domain": "company",
  "control": "grasp",
  "experiment": {"prediction": "不碰两周会出小错", "review_on": "2026-11-07"}
}"#;

#[test]
fn record_full_has_nothing_missing() {
    let dir = lab();
    let outcome = record(&dir, FULL);
    assert_eq!(outcome["missing"].as_array().expect("数组").len(), 0);
    assert_eq!(
        outcome["next_question"],
        "到复看日回填结果：实际发生的事，比任何推理都算数。"
    );
    assert_eq!(outcome["recurring"].as_array().expect("数组").len(), 0);
}

#[test]
fn record_stores_chinese_labels_for_english_keys() {
    let dir = lab();
    record(&dir, FULL);
    let stored = ledger(&dir);
    let distortions = stored["reflections"][0]["distortions"]
        .as_array()
        .expect("数组");
    assert_eq!(distortions[0], "个人化");
    assert_eq!(distortions[1], "灾难化");
}

#[test]
fn record_asks_the_first_missing_field_first() {
    let dir = lab();
    let outcome = record(&dir, r#"{"situation":"又接了活","thought":"这事没我不行"}"#);
    assert_eq!(outcome["missing"][0], "emotion");
    assert_eq!(
        outcome["next_question"],
        "这件事让你感到什么？强度大概几分（0-100）？"
    );
}

#[test]
fn record_without_situation_exits_2() {
    let dir = lab();
    let out = record_fails(&dir, r#"{"thought":"这事没我不行"}"#);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr_of(&out).contains("situation"), "{}", stderr_of(&out));
}

#[test]
fn record_without_thought_exits_2() {
    let dir = lab();
    let out = record_fails(&dir, r#"{"situation":"又接了活"}"#);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr_of(&out).contains("thought"), "{}", stderr_of(&out));
}

#[test]
fn record_rejects_unknown_distortion() {
    let dir = lab();
    let out = record_fails(
        &dir,
        r#"{"situation":"又接了活","thought":"没我不行","distortions":["瞎猜"]}"#,
    );
    assert_eq!(out.status.code(), Some(2));
    let text = stderr_of(&out);
    assert!(text.contains("个人化"), "应列出可用标签：{text}");
}

#[test]
fn record_rejects_intensity_out_of_range() {
    let dir = lab();
    let out = record_fails(
        &dir,
        r#"{"situation":"又接了活","thought":"没我不行","emotion":"焦虑","intensity":120}"#,
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr_of(&out).contains("0-100"), "{}", stderr_of(&out));
}

#[test]
fn record_rejects_bad_review_date() {
    let dir = lab();
    let out = record_fails(
        &dir,
        r#"{"situation":"x","thought":"y","experiment":{"prediction":"会出事","review_on":"11月7日"}}"#,
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr_of(&out).contains("YYYY-MM-DD"),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn recurring_lists_distortions_from_two_records() {
    let dir = lab();
    record(
        &dir,
        r#"{"situation":"a","thought":"b","distortions":["个人化"]}"#,
    );
    let outcome = record(
        &dir,
        r#"{"situation":"c","thought":"d","distortions":["个人化","灾难化"]}"#,
    );
    let recurring = outcome["recurring"].as_array().expect("数组");
    assert_eq!(recurring.len(), 1);
    assert_eq!(recurring[0]["label"], "个人化");
    assert_eq!(recurring[0]["count"], 2);
}

#[test]
fn list_empty_ledger_is_quiet() {
    let dir = lab();
    let outcome = list(&dir, &[]);
    assert_eq!(outcome["due"].as_array().expect("数组").len(), 0);
    assert_eq!(outcome["open"].as_array().expect("数组").len(), 0);
    assert_eq!(outcome["recurring"].as_array().expect("数组").len(), 0);
}

#[test]
fn list_marks_unfinished_records_open() {
    let dir = lab();
    record(&dir, r#"{"situation":"又接了活","thought":"没我不行"}"#);
    let outcome = list(&dir, &[]);
    let open = outcome["open"].as_array().expect("数组");
    assert_eq!(open.len(), 1);
    assert_eq!(open[0]["situation"], "又接了活");
    assert_eq!(open[0]["missing"][0], "emotion");
}

#[test]
fn list_due_only_skips_future_experiments() {
    let dir = lab();
    record(
        &dir,
        r#"{"situation":"还没到","thought":"x","experiment":{"prediction":"会出事","review_on":"2999-01-01"}}"#,
    );
    let outcome = list(&dir, &["--due"]);
    assert_eq!(outcome["due"].as_array().expect("数组").len(), 0);
    assert_eq!(outcome["open"].as_array().expect("数组").len(), 0);
}

#[test]
fn list_due_returns_past_experiments() {
    let dir = lab();
    let outcome = record(
        &dir,
        r#"{"situation":"让团队自己谈","thought":"肯定谈砸","experiment":{"prediction":"两周内出小错","review_on":"2020-01-01"}}"#,
    );
    let id = outcome["id"].as_str().expect("id").to_string();
    let due = list(&dir, &["--due"]);
    assert_eq!(due["due"].as_array().expect("数组").len(), 1);
    assert_eq!(due["due"][0]["id"], id);
    assert_eq!(due["due"][0]["prediction"], "两周内出小错");
}

#[test]
fn observe_records_result_and_emotion() {
    let dir = lab();
    let outcome = record(
        &dir,
        r#"{"situation":"让团队自己谈","thought":"肯定谈砸","experiment":{"prediction":"两周内出小错","review_on":"2020-01-01"}}"#,
    );
    let id = outcome["id"].as_str().expect("id").to_string();
    let out = cmd(&dir)
        .args([
            "health",
            "reflect",
            "--observe",
            &id,
            "--result",
            "只错了一个报价点，当天改回来了",
            "--emotion",
            "30",
            "--json",
        ])
        .output()
        .expect("运行二进制");
    assert!(out.status.success(), "{}", stderr_of(&out));
    let observed: Value = serde_json::from_str(&stdout_of(&out)).expect("JSON");
    assert_eq!(observed["id"], id);
    assert_eq!(observed["emotion_after"], 30);

    let stored = ledger(&dir);
    assert_eq!(
        stored["reflections"][0]["experiment"]["result"],
        "只错了一个报价点，当天改回来了"
    );
}

#[test]
fn observe_unknown_id_exits_3() {
    let dir = lab();
    let out = cmd(&dir)
        .args([
            "health",
            "reflect",
            "--observe",
            "ref_ZZZZ",
            "--result",
            "x",
        ])
        .output()
        .expect("运行二进制");
    assert_eq!(out.status.code(), Some(3));
    assert!(stderr_of(&out).contains("NOT_FOUND"), "{}", stderr_of(&out));
}

#[test]
fn observe_without_experiment_exits_2() {
    let dir = lab();
    let outcome = record(&dir, r#"{"situation":"a","thought":"b"}"#);
    let id = outcome["id"].as_str().expect("id").to_string();
    let out = cmd(&dir)
        .args(["health", "reflect", "--observe", &id, "--result", "x"])
        .output()
        .expect("运行二进制");
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr_of(&out).contains("行为实验"), "{}", stderr_of(&out));
}

#[test]
fn conflicting_modes_exit_2() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["health", "reflect", "--stdin", "--list"])
        .output()
        .expect("运行二进制");
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr_of(&out).contains("四选一"), "{}", stderr_of(&out));
}

#[test]
fn due_without_list_exits_2() {
    let dir = lab();
    let out = cmd(&dir)
        .args(["health", "reflect", "--stdin", "--due"])
        .write_stdin("{}")
        .output()
        .expect("运行二进制");
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr_of(&out).contains("--due"), "{}", stderr_of(&out));
}

#[test]
fn list_human_collapses_old_open_records() {
    let dir = lab();
    for index in 0..5 {
        let body = format!(r#"{{"situation":"事件{index}","thought":"念头{index}"}}"#);
        record(&dir, &body);
    }
    let out = cmd(&dir)
        .args(["health", "reflect", "--list"])
        .output()
        .expect("运行二进制");
    let text = stdout_of(&out);
    assert!(text.contains("未填完 5 条（只列最近 3 条）"), "{text}");
    assert!(text.contains("另有 2 条未列出"), "{text}");
    assert!(text.contains("事件4"), "应列出最新的记录：{text}");
    assert!(!text.contains("事件0"), "不应列出最旧的记录：{text}");
}

#[test]
fn id_shows_one_record() {
    let dir = lab();
    let outcome = record(&dir, FULL);
    let id = outcome["id"].as_str().expect("id").to_string();
    let out = cmd(&dir)
        .args(["health", "reflect", "--id", &id, "--json"])
        .output()
        .expect("运行二进制");
    assert!(out.status.success(), "{}", stderr_of(&out));
    let one: Value = serde_json::from_str(&stdout_of(&out)).expect("JSON");
    assert_eq!(one["situation"], "周三亲自去谈供应商");
    assert_eq!(one["missing"].as_array().expect("数组").len(), 0);
}
