//! 编译流水线
//!
//! 管理编译状态机、执行编译流程、处理错误恢复。
//!
//! `Pipeline::run` 的函数体已按 02-stage-contract §改动清单替换为
//! `Program { kind: SingleFile }` 构造 + `Driver` 调用（WBS 4.1.3）；
//! 各阶段实现迁入 `crate::driver` 的 `Stage` 臂。本文件保留
//! `PipelineError` / `CompilationResult` / `Pipeline` 外观——对外签名
//! 不变，单文件诊断集与退出码逐字节相同（验收判据 C2）。

use crate::driver::{Driver, Program, ProgramKind, Unit};
use crate::middle;
use crate::util::diagnostic::Diagnostic;
use std::fmt;

use super::config::CompileConfig;

/// 管道错误类型
#[derive(Debug, Clone)]
pub enum PipelineError {
    /// 词法/解析错误（携带原始诊断，保留错误码与 span）
    LexParse(Diagnostic),
    /// 类型检查错误
    TypeCheck(Diagnostic),
    /// IR 生成错误
    IRGeneration(String),
    ProofExecution(Diagnostic),
    /// 编排层内部错误（Driver 未接线阶段等结构性不可达路径的防御通道）
    Internal(String),
}

impl fmt::Display for PipelineError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            PipelineError::LexParse(msg) => write!(f, "{}", msg),
            PipelineError::TypeCheck(err) => write!(f, "{}", err),
            PipelineError::IRGeneration(msg) => write!(f, "{}", msg),
            PipelineError::ProofExecution(msg) => write!(f, "{}", msg),
            PipelineError::Internal(msg) => write!(f, "{}", msg),
        }
    }
}

impl PipelineError {
    /// 获取诊断信息（如果是类型检查错误）
    pub fn diagnostic(&self) -> Option<Diagnostic> {
        match self {
            PipelineError::LexParse(err) => Some(err.clone()),
            PipelineError::TypeCheck(err) => Some(err.clone()),
            PipelineError::ProofExecution(err) => Some(err.clone()),
            _ => None,
        }
    }
}

/// 编译结果
#[derive(Debug, Clone, Default)]
pub struct CompilationResult {
    /// 生成的 IR
    pub ir: Option<middle::ModuleIR>,
    /// 错误数量
    pub error_count: usize,
    /// 警告数量
    pub warning_count: usize,
    /// 总耗时（毫秒）
    pub total_duration_ms: u64,
    /// 错误
    pub errors: Vec<PipelineError>,
    /// 警告诊断（#321 M2 结构化，含 severity/code/span）
    pub warnings: Vec<Diagnostic>,
}

impl CompilationResult {
    /// 创建成功结果
    pub fn success(
        ir: middle::ModuleIR,
        total_ms: u64,
        warnings: Vec<Diagnostic>,
    ) -> Self {
        Self {
            ir: Some(ir),
            error_count: 0,
            warning_count: warnings.len(),
            total_duration_ms: total_ms,
            errors: Vec::new(),
            warnings,
        }
    }

    /// 创建失败结果
    pub fn failed(
        errors: Vec<PipelineError>,
        total_ms: u64,
    ) -> Self {
        Self {
            ir: None,
            error_count: errors.len(),
            warning_count: 0,
            total_duration_ms: total_ms,
            errors,
            warnings: Vec::new(),
        }
    }

    /// 是否成功
    pub fn is_success(&self) -> bool {
        self.error_count == 0 && self.ir.is_some()
    }
}

/// 编译流水线
pub struct Pipeline {
    /// 配置
    config: CompileConfig,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new(CompileConfig::default())
    }
}

impl fmt::Debug for Pipeline {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("Pipeline")
            .field("config", &self.config)
            .finish()
    }
}

impl Pipeline {
    /// 创建新流水线
    pub fn new(config: CompileConfig) -> Self {
        Self { config }
    }

    /// 获取配置
    #[inline]
    pub fn config(&self) -> &CompileConfig {
        &self.config
    }

    /// 运行完整编译流程
    ///
    /// 函数体 = `Program { kind: SingleFile }` 构造 + Driver 调用
    /// （02 §改动清单，WBS 4.1.3）；阶段实现见 `crate::driver` 的
    /// `Stage::dispatch` 各臂。
    pub fn run(
        &mut self,
        source_name: &str,
        source: &str,
    ) -> CompilationResult {
        let start_time = crate::util::time_compat::Instant::now();
        let program = Program::new(
            ProgramKind::SingleFile,
            vec![Unit::new(source_name, source_name, source)],
            self.config.clone(),
        );
        let mut result = match Driver::new().run(program) {
            Ok(outcome) => outcome.result,
            // SingleFile 阶段表全接线且恒为 1 单元——结构性不可达；
            // Pipeline 是 pub 外观：显式 Internal 错误优于 panic
            Err(e) => CompilationResult::failed(vec![PipelineError::Internal(e.to_string())], 0),
        };
        result.total_duration_ms = start_time.elapsed().as_millis() as u64;
        result
    }
}
