//! 单文件 JSON 存储：数据目录默认 `data/`（环境变量 `QTFOUNDER_LAB_DATA` 覆盖整个数据目录），
//! 意图与澄清记录落 `clarify.json`，规则落 `rules.md`。不引 SQLite（离线无 bundled sqlite）。

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::AppError;

const DATA_FILE: &str = "clarify.json";
const RULES_FILE: &str = "rules.md";

/// 一次澄清记录（这次哪里不对）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clarification {
    pub id: String,
    pub at: String,
    pub before: String,
    pub actual: String,
    pub prompt: String,
}

/// 一件事。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub id: String,
    pub task: String,
    pub at: String,
    #[serde(default)]
    pub clarifications: Vec<Clarification>,
}

/// 重复 ≥2 次的差（规则候选），`key` 是 `actual` 归一化后的键。
#[derive(Debug, Clone)]
pub struct CandidateGroup {
    pub key: String,
    pub ids: Vec<String>,
}

/// 数据文件的全部内容：意图层 + 已固化重复差的 id（元数据，不是第三个领域对象）。
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Store {
    #[serde(default)]
    pub intents: Vec<Intent>,
    /// 已由 `rules.md` 固化的重复差（澄清 id），立规则后不再计入新规则的 `source`
    #[serde(default)]
    pub processed_clarifications: Vec<String>,
}

/// 数据目录：`QTFOUNDER_LAB_DATA` 覆盖整个数据目录，默认 `data/`。
pub fn data_dir() -> PathBuf {
    match std::env::var_os("QTFOUNDER_LAB_DATA") {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => PathBuf::from("data"),
    }
}

pub fn data_file() -> PathBuf {
    data_dir().join(DATA_FILE)
}

pub fn rules_file() -> PathBuf {
    data_dir().join(RULES_FILE)
}

fn io_error(action: &str, path: &Path, err: std::io::Error) -> AppError {
    AppError::usage(format!("无法{action} {}：{err}", path.display()))
}

fn ensure_parent(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|err| io_error("创建目录", parent, err))?;
    }
    Ok(())
}

impl Store {
    /// 读数据文件；文件不存在按空库处理。
    pub fn load() -> Result<Self, AppError> {
        let path = data_file();
        match fs::read_to_string(&path) {
            Ok(text) if text.trim().is_empty() => Ok(Store::default()),
            Ok(text) => serde_json::from_str(&text).map_err(|err| {
                AppError::usage(format!("数据文件解析失败 {}：{err}", path.display()))
            }),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Store::default()),
            Err(err) => Err(io_error("读取", &path, err)),
        }
    }

    /// 写数据文件（先建目录）。
    pub fn save(&self) -> Result<(), AppError> {
        let path = data_file();
        ensure_parent(&path)?;
        let mut text = serde_json::to_string_pretty(self)
            .map_err(|err| AppError::usage(format!("数据序列化失败：{err}")))?;
        text.push('\n');
        fs::write(&path, text).map_err(|err| io_error("写入", &path, err))
    }

    /// 按首次出现顺序给出本意图下重复 ≥2 次的差。
    pub fn candidates(&self) -> Vec<CandidateGroup> {
        self.intents.iter().flat_map(Intent::candidates).collect()
    }

    /// 尚未被 `rules.md` 固化的重复差 id（跨全部意图，首次出现顺序）。
    pub fn unprocessed_duplicate_ids(&self) -> Vec<String> {
        self.candidates()
            .into_iter()
            .flat_map(|group| group.ids)
            .filter(|id| !self.processed_clarifications.contains(id))
            .collect()
    }

    /// 去重键 sha256(before + actual) 命中的既有记录。
    pub fn find_by_dedup_key(&self, key: &str) -> Option<(String, &Clarification)> {
        self.intents.iter().find_map(|intent| {
            intent
                .clarifications
                .iter()
                .find(|cl| dedup_key(&cl.before, &cl.actual) == key)
                .map(|cl| (intent.id.clone(), cl))
        })
    }

    pub fn intent_exists(&self, id: &str) -> bool {
        self.intents.iter().any(|intent| intent.id == id)
    }

    pub fn clarification_exists(&self, id: &str) -> bool {
        self.intents
            .iter()
            .any(|intent| intent.clarifications.iter().any(|cl| cl.id == id))
    }
}

impl Intent {
    /// 本意图下 `actual` 归一化后重复 ≥2 次的差。
    pub fn candidates(&self) -> Vec<CandidateGroup> {
        let mut groups: Vec<CandidateGroup> = Vec::new();
        for cl in &self.clarifications {
            let key = actual_key(&cl.actual);
            match groups.iter_mut().find(|group| group.key == key) {
                Some(group) => group.ids.push(cl.id.clone()),
                None => groups.push(CandidateGroup {
                    key,
                    ids: vec![cl.id.clone()],
                }),
            }
        }
        groups.retain(|group| group.ids.len() >= 2);
        groups
    }
}

/// 生成 id：`int_01J9` / `clr_01J9` 形如，前缀 + 4 位 Crockford Base32（sha256 取前 20 位）。
pub fn new_id(prefix: &str, is_taken: impl Fn(&str) -> bool) -> String {
    const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut attempt: u32 = 0;
    loop {
        let mut hasher = Sha256::new();
        hasher.update(prefix.as_bytes());
        hasher.update(nanos.to_be_bytes());
        hasher.update(COUNTER.fetch_add(1, Ordering::Relaxed).to_be_bytes());
        hasher.update(attempt.to_be_bytes());
        let digest = hasher.finalize();
        let bits = (u32::from(digest[0]) << 12)
            | (u32::from(digest[1]) << 4)
            | (u32::from(digest[2]) >> 4);

        let mut id = String::with_capacity(prefix.len() + 5);
        id.push_str(prefix);
        id.push('_');
        for shift in [15, 10, 5, 0] {
            id.push(ALPHABET[((bits >> shift) & 31) as usize] as char);
        }
        if !is_taken(&id) {
            return id;
        }
        attempt += 1;
        assert!(attempt < 1024, "id 生成碰撞过多，检查数据文件");
    }
}

/// 幂等去重键：sha256(before + actual)，十六进制小写。
pub fn dedup_key(before: &str, actual: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(before.as_bytes());
    hasher.update(actual.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// 归一化：小写、去标点、去空白——只留字母数字（含中文），同类差归一个键。
pub fn actual_key(actual: &str) -> String {
    actual
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// 当前时间，ISO 8601（UTC，秒级）。
pub fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    iso8601(secs)
}

fn iso8601(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// 把 UTC 天数换算成年月日（Howard Hinnant 的 civil calendar 算法）。
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (y + i64::from(m <= 2), m, d)
}

/// 向 `rules.md` 追加一行规则：`- <规则文本>`，返回落盘路径。
pub fn append_rule_line(rule: &str) -> Result<PathBuf, AppError> {
    let path = rules_file();
    ensure_parent(&path)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| io_error("写入", &path, err))?;
    writeln!(file, "- {rule}").map_err(|err| io_error("写入", &path, err))?;
    Ok(path)
}
