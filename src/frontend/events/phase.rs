//! 编译阶段事件

use super::{base::CompilationPhase, Event, EventMetadata, EventType};

/// 词法分析开始事件
#[derive(Debug, Clone)]
pub struct LexingStart {
    source_name: String,
    source_length: usize,
    metadata: EventMetadata,
}

impl LexingStart {
    pub fn new(
        source_name: impl Into<String>,
        source_length: usize,
    ) -> Self {
        Self {
            source_name: source_name.into(),
            source_length,
            metadata: EventMetadata::default(),
        }
    }

    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    pub fn source_length(&self) -> usize {
        self.source_length
    }
}

impl Event for LexingStart {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "LexingStart"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// 词法分析完成事件
#[derive(Debug, Clone)]
pub struct LexingComplete {
    token_count: usize,
    duration_ms: u64,
    metadata: EventMetadata,
}

impl LexingComplete {
    pub fn new(
        token_count: usize,
        duration_ms: u64,
    ) -> Self {
        Self {
            token_count,
            duration_ms,
            metadata: EventMetadata::default(),
        }
    }

    pub fn token_count(&self) -> usize {
        self.token_count
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }
}

impl Event for LexingComplete {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "LexingComplete"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// 语法分析开始事件
#[derive(Debug, Clone)]
pub struct ParsingStart {
    token_count: usize,
    metadata: EventMetadata,
}

impl ParsingStart {
    pub fn new(token_count: usize) -> Self {
        Self {
            token_count,
            metadata: EventMetadata::default(),
        }
    }

    pub fn token_count(&self) -> usize {
        self.token_count
    }
}

impl Event for ParsingStart {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "ParsingStart"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// 语法分析完成事件
#[derive(Debug, Clone)]
pub struct ParsingComplete {
    ast_node_count: usize,
    duration_ms: u64,
    metadata: EventMetadata,
}

impl ParsingComplete {
    pub fn new(
        ast_node_count: usize,
        duration_ms: u64,
    ) -> Self {
        Self {
            ast_node_count,
            duration_ms,
            metadata: EventMetadata::default(),
        }
    }

    pub fn ast_node_count(&self) -> usize {
        self.ast_node_count
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }
}

impl Event for ParsingComplete {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "ParsingComplete"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// 类型检查开始事件
#[derive(Debug, Clone)]
pub struct TypeCheckingStart {
    module_name: String,
    declaration_count: usize,
    metadata: EventMetadata,
}

impl TypeCheckingStart {
    pub fn new(
        module_name: impl Into<String>,
        declaration_count: usize,
    ) -> Self {
        Self {
            module_name: module_name.into(),
            declaration_count,
            metadata: EventMetadata::default(),
        }
    }

    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    pub fn declaration_count(&self) -> usize {
        self.declaration_count
    }
}

impl Event for TypeCheckingStart {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "TypeCheckingStart"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// 类型检查完成事件
#[derive(Debug, Clone)]
pub struct TypeCheckingComplete {
    inferred_types: usize,
    errors: usize,
    warnings: usize,
    duration_ms: u64,
    metadata: EventMetadata,
}

impl TypeCheckingComplete {
    pub fn new(
        inferred_types: usize,
        errors: usize,
        warnings: usize,
        duration_ms: u64,
    ) -> Self {
        Self {
            inferred_types,
            errors,
            warnings,
            duration_ms,
            metadata: EventMetadata::default(),
        }
    }

    pub fn inferred_types(&self) -> usize {
        self.inferred_types
    }

    pub fn error_count(&self) -> usize {
        self.errors
    }

    pub fn warning_count(&self) -> usize {
        self.warnings
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }
}

impl Event for TypeCheckingComplete {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "TypeCheckingComplete"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// IR 生成开始事件
#[derive(Debug, Clone)]
pub struct IRGenerationStart {
    function_count: usize,
    metadata: EventMetadata,
}

impl IRGenerationStart {
    pub fn new(function_count: usize) -> Self {
        Self {
            function_count,
            metadata: EventMetadata::default(),
        }
    }

    pub fn function_count(&self) -> usize {
        self.function_count
    }
}

impl Event for IRGenerationStart {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "IRGenerationStart"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// IR 生成完成事件
#[derive(Debug, Clone)]
pub struct IRGenerationComplete {
    ir_size: usize,
    function_count: usize,
    duration_ms: u64,
    metadata: EventMetadata,
}

impl IRGenerationComplete {
    pub fn new(
        ir_size: usize,
        function_count: usize,
        duration_ms: u64,
    ) -> Self {
        Self {
            ir_size,
            function_count,
            duration_ms,
            metadata: EventMetadata::default(),
        }
    }

    pub fn ir_size(&self) -> usize {
        self.ir_size
    }

    pub fn function_count(&self) -> usize {
        self.function_count
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }
}

impl Event for IRGenerationComplete {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "IRGenerationComplete"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// 完整编译开始事件
#[derive(Debug, Clone)]
pub struct CompilationStart {
    source_name: String,
    source_length: usize,
    incremental: bool,
    metadata: EventMetadata,
}

impl CompilationStart {
    pub fn new(
        source_name: impl Into<String>,
        source_length: usize,
        incremental: bool,
    ) -> Self {
        Self {
            source_name: source_name.into(),
            source_length,
            incremental,
            metadata: EventMetadata::default(),
        }
    }

    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    pub fn source_length(&self) -> usize {
        self.source_length
    }

    pub fn is_incremental(&self) -> bool {
        self.incremental
    }
}

impl Event for CompilationStart {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "CompilationStart"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// 完整编译完成事件
#[derive(Debug, Clone)]
pub struct CompilationComplete {
    success: bool,
    total_duration_ms: u64,
    phases: Vec<(CompilationPhase, u64)>,
    metadata: EventMetadata,
}

impl CompilationComplete {
    pub fn new(
        success: bool,
        total_duration_ms: u64,
        phases: Vec<(CompilationPhase, u64)>,
    ) -> Self {
        Self {
            success,
            total_duration_ms,
            phases,
            metadata: EventMetadata::default(),
        }
    }

    pub fn success(&self) -> bool {
        self.success
    }

    pub fn total_duration_ms(&self) -> u64 {
        self.total_duration_ms
    }

    pub fn phases(&self) -> &[(CompilationPhase, u64)] {
        &self.phases
    }
}

impl Event for CompilationComplete {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "CompilationComplete"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}
