//! 语义化版本解析入口（RFC-014 Phase 3：`semver` crate 替换手写解析器）
//!
//! 类型直接复用 [`semver::Version`] / [`semver::VersionReq`]；本模块只保留
//! 三类自有逻辑：
//! - 入口宽容化：manifest/tag 中的两段（`1.2`）与单段（`1`）版本补齐为三段
//! - 裸版本语义：`"1.2.3"` 作为**精确**要求（`=1.2.3`），不采用 Cargo 的
//!   caret 默认——这是 `add <pkg> <ver>` 文档化行为
//! - 兼容性判定：`is_compatible` 用区间交集替代旧实现的 10 万次枚举
//!   （旧实现对 `>=100.0.0` 一侧必然误判冲突）
//!
//! 支持的要求格式（经 [`semver::VersionReq`]）：
//! - `*`、`1.*`、`1.2.*` — 通配
//! - `^1.2.3` / `~1.2.3` — caret / tilde
//! - `>=1.0.0` `>1.0.0` `<=1.0.0` `<1.0.0` `=1.0.0` — 比较器
//! - 逗号分隔的组合（如 `>=1.2.3, <2.0.0`）

use semver::{Comparator, Op, Version, VersionReq};

use crate::package::error::{PackageError, PackageResult};

/// 解析单个版本号（tag / lock 中的 `version` 字段等）
///
/// 宽容化：`1.2` → `1.2.0`，`1` → `1.0.0`（预发布/构建段原样保留）。
pub fn parse_version(s: &str) -> PackageResult<Version> {
    let s = s.trim();
    Version::parse(&pad_version(s)?)
        .map_err(|e| PackageError::InvalidManifest(format!("无效的版本号 '{}': {}", s, e)))
}

/// 解析版本要求字符串
///
/// 裸版本（`1.2.3`、`1.2`）按精确匹配处理；其余格式透传给
/// [`semver::VersionReq`]（caret/tilde/比较器/通配/组合）。
pub fn parse_version_req(s: &str) -> PackageResult<VersionReq> {
    let s = s.trim();

    if s == "*" || s.is_empty() {
        return VersionReq::parse("*").map_err(req_error(s));
    }

    // 裸版本（数字开头、无通配与组合符）→ 精确匹配，保留 `add` 的既有语义
    let first = s.chars().next().unwrap_or('*');
    if first.is_ascii_digit() && !s.contains('*') && !s.contains(',') {
        let padded = pad_version(s)?;
        return VersionReq::parse(&format!("={}", padded)).map_err(req_error(s));
    }

    VersionReq::parse(s).map_err(req_error(s))
}

fn req_error(input: &str) -> impl Fn(semver::Error) -> PackageError + '_ {
    move |e| PackageError::InvalidManifest(format!("无效的版本要求 '{}': {}", input, e))
}

/// 把不足三段的数字版本补齐为三段（预发布/构建段原样保留）
fn pad_version(s: &str) -> PackageResult<String> {
    // 预发布段从第一个 '-' 起（预发布标识内部的 '-' 不影响主段切分）；
    // 构建元数据从 '+' 起
    let core_end = s.find(['-', '+']).unwrap_or(s.len());
    let (core, suffix) = s.split_at(core_end);

    let core_len = core.split('.').count();
    if core_len > 3 || core.is_empty() {
        return Ok(s.to_string()); // 交由 semver crate 报错
    }
    let missing = 3usize.saturating_sub(core_len);
    let pad = ".0".repeat(missing);
    Ok(format!("{}{}{}", core, pad, suffix))
}

/// 从候选版本中选出满足要求的最高版本
pub fn select_best<'a>(
    req: &VersionReq,
    versions: &'a [Version],
) -> Option<&'a Version> {
    let mut candidates: Vec<&Version> = versions.iter().filter(|v| req.matches(v)).collect();
    candidates.sort_by(|a, b| b.cmp(a));
    candidates.into_iter().next()
}

/// 两个版本要求是否兼容（存在同时满足两者的版本）
///
/// 将每个要求折算为半开区间后取交集。预发布版本存在 crate 级匹配限制
/// （无预发布段的比较器不匹配预发布版本），此处按纯数值序判定，对预发布
/// 边界可能给出宽松结论——冲突检测是建议性的，可接受。
pub fn is_compatible(
    a: &VersionReq,
    b: &VersionReq,
) -> bool {
    intersect(&req_interval(a), &req_interval(b)).is_some()
}

/// 闭/开边界标注的版本区间
#[derive(Debug, Clone)]
struct Interval {
    lo: Option<(Version, bool)>,
    hi: Option<(Version, bool)>,
}

const INCLUSIVE: bool = true;
const EXCLUSIVE: bool = false;

const FULL_RANGE: Interval = Interval { lo: None, hi: None };

/// 把一个版本要求折算为区间（各比较器区间的交）
fn req_interval(req: &VersionReq) -> Interval {
    if req.comparators.is_empty() {
        return FULL_RANGE; // `*`
    }

    let mut acc = FULL_RANGE;
    for cmp in &req.comparators {
        let Some(next) = intersect(&acc, &comparator_interval(cmp)) else {
            return Interval {
                lo: Some((Version::new(1, 0, 0), INCLUSIVE)),
                hi: Some((Version::new(0, 0, 0), INCLUSIVE)), // 恒空区间
            };
        };
        acc = next;
    }
    acc
}

/// 单个比较器 → 区间
fn comparator_interval(cmp: &Comparator) -> Interval {
    let version = |minor: Option<u64>, patch: Option<u64>| {
        Version::new(cmp.major, minor.unwrap_or(0), patch.unwrap_or(0))
    };

    match cmp.op {
        Op::Exact => Interval {
            lo: Some((
                Version::new(cmp.major, cmp.minor.unwrap_or(0), cmp.patch.unwrap_or(0)),
                INCLUSIVE,
            )),
            hi: Some((
                Version::new(cmp.major, cmp.minor.unwrap_or(0), cmp.patch.unwrap_or(0)),
                INCLUSIVE,
            )),
        },
        Op::Greater => Interval {
            lo: Some((version(cmp.minor, cmp.patch), EXCLUSIVE)),
            hi: None,
        },
        Op::GreaterEq => Interval {
            lo: Some((version(cmp.minor, cmp.patch), INCLUSIVE)),
            hi: None,
        },
        Op::Less => Interval {
            lo: None,
            hi: Some((version(cmp.minor, cmp.patch), EXCLUSIVE)),
        },
        Op::LessEq => Interval {
            lo: None,
            hi: Some((version(cmp.minor, cmp.patch), INCLUSIVE)),
        },
        Op::Tilde => Interval {
            lo: Some((version(cmp.minor, cmp.patch), INCLUSIVE)),
            hi: Some((tilde_upper(cmp), EXCLUSIVE)),
        },
        Op::Caret => Interval {
            lo: Some((version(cmp.minor, cmp.patch), INCLUSIVE)),
            hi: Some((caret_upper(cmp), EXCLUSIVE)),
        },
        Op::Wildcard => Interval {
            lo: Some((version(cmp.minor, cmp.patch), INCLUSIVE)),
            hi: Some((
                match cmp.minor {
                    None => Version::new(cmp.major + 1, 0, 0),
                    Some(minor) => Version::new(cmp.major, minor + 1, 0),
                },
                EXCLUSIVE,
            )),
        },
        _ => FULL_RANGE, // 未识别的比较器按全区间宽松处理
    }
}

/// `~x.y.z` 的排他上界：`~1.2` → `1.3.0`，`~1` → `2.0.0`
fn tilde_upper(cmp: &Comparator) -> Version {
    match cmp.minor {
        None => Version::new(cmp.major + 1, 0, 0),
        Some(minor) => Version::new(cmp.major, minor + 1, 0),
    }
}

/// `^x.y.z` 的排他上界：major>0 进主版本，major=0 进次版本，major=minor=0 进补丁
fn caret_upper(cmp: &Comparator) -> Version {
    match cmp.minor {
        None => Version::new(cmp.major + 1, 0, 0), // ^1 → <2.0.0
        Some(minor) => match (cmp.major, minor, cmp.patch) {
            (0, 0, Some(patch)) => Version::new(0, 0, patch + 1), // ^0.0.3 → <0.0.4
            (0, 0, None) => Version::new(0, 1, 0),                // ^0.0 → <0.1.0
            (0, m, _) => Version::new(0, m + 1, 0),               // ^0.2.3 → <0.3.0
            (major, _, _) => Version::new(major + 1, 0, 0),       // ^1.2.3 → <2.0.0
        },
    }
}

/// 区间交集；空则返回 None
fn intersect(
    a: &Interval,
    b: &Interval,
) -> Option<Interval> {
    let lo = max_bound(&a.lo, &b.lo);
    let hi = min_bound(&a.hi, &b.hi);

    if let (Some((lv, li)), Some((hv, hi_inc))) = (&lo, &hi) {
        match lv.cmp(hv) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal if *li && *hi_inc => {}
            // lo > hi，或 lo == hi 但任一端排他 → 空区间
            _ => return None,
        }
    }

    Some(Interval { lo, hi })
}

/// 取更严格（更大）的下界；相等时排他端优先
fn max_bound(
    a: &Option<(Version, bool)>,
    b: &Option<(Version, bool)>,
) -> Option<(Version, bool)> {
    match (a, b) {
        (None, other) | (other, None) => other.clone(),
        (Some((va, ia)), Some((vb, ib))) => match va.cmp(vb) {
            std::cmp::Ordering::Greater => Some((va.clone(), *ia)),
            std::cmp::Ordering::Less => Some((vb.clone(), *ib)),
            std::cmp::Ordering::Equal => Some((va.clone(), *ia && *ib)),
        },
    }
}

/// 取更严格（更小）的上界；相等时排他端优先
fn min_bound(
    a: &Option<(Version, bool)>,
    b: &Option<(Version, bool)>,
) -> Option<(Version, bool)> {
    match (a, b) {
        (None, other) | (other, None) => other.clone(),
        (Some((va, ia)), Some((vb, ib))) => match va.cmp(vb) {
            std::cmp::Ordering::Less => Some((va.clone(), *ia)),
            std::cmp::Ordering::Greater => Some((vb.clone(), *ib)),
            std::cmp::Ordering::Equal => Some((va.clone(), *ia && *ib)),
        },
    }
}
