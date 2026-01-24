use std::sync::Arc;
/// 调试面板组件
///
/// 显示调试信息，包括调用栈、变量和性能统计
use ratatui::{
    layout::{Rect, Margin},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::backends::dev::tui_repl::engine::IncrementalCompiler;

/// 调试面板
pub struct DebugPanel {
    /// 是否显示调试信息
    visible: bool,
    /// 选中的标签页
    selected_tab: DebugTab,
}

impl DebugPanel {
    /// 创建新的调试面板
    pub fn new() -> Self {
        Self {
            visible: true,
            selected_tab: DebugTab::CallStack,
        }
    }

    /// 切换可见性
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// 设置可见性
    pub fn set_visible(
        &mut self,
        visible: bool,
    ) {
        self.visible = visible;
    }

    /// 切换标签页
    pub fn next_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            DebugTab::CallStack => DebugTab::Variables,
            DebugTab::Variables => DebugTab::Performance,
            DebugTab::Performance => DebugTab::Memory,
            DebugTab::Memory => DebugTab::CallStack,
        };
    }

    /// 渲染调试面板
    pub fn render(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        compiler: &Arc<IncrementalCompiler>,
    ) {
        if !self.visible {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .title(" Debug ");

        f.render_widget(block, area);

        let inner_area = area.inner(&Margin::default());

        // 获取调试信息
        let debug_info = self.get_debug_info(compiler);

        let paragraph = Paragraph::new(debug_info)
            .style(Style::default().fg(Color::Green))
            .scroll((0, 0));

        f.render_widget(paragraph, inner_area);
    }

    /// 获取调试信息
    fn get_debug_info(
        &self,
        compiler: &Arc<IncrementalCompiler>,
    ) -> String {
        let stats = compiler.stats();
        let profiler = compiler.profiler();

        let stats = stats.read().unwrap();
        let profiler = profiler.read().unwrap();
        let report = profiler.generate_report();

        let mut info = String::new();

        info.push_str(&format!(
            "Statements: {}\n",
            compiler.get_module_summary().statement_count
        ));
        info.push_str(&format!(
            "Symbols: {}\n",
            compiler.get_module_summary().symbol_count
        ));
        info.push_str("\n[Compilation]\n");
        info.push_str(&format!("Total: {}\n", stats.total_compilations));
        info.push_str(&format!("Success: {}\n", stats.successful_compilations));
        info.push_str(&format!("Failed: {}\n", stats.failed_compilations));

        if let Some(avg_time) = report.compilation_stats.average_time {
            info.push_str(&format!(
                "Avg Time: {:.2}ms\n",
                avg_time.as_secs_f64() * 1000.0
            ));
        }

        info.push_str("\n[Cache]\n");
        info.push_str(&format!("Hit Rate: {:.1}%\n", report.cache_stats.hit_rate));

        info.push_str("\n[Performance]\n");
        info.push_str(&format!(
            "Total Executions: {}\n",
            report.execution_stats.total_executions
        ));

        if let Some(ref most_expensive) = report.execution_stats.most_expensive_function {
            info.push_str(&format!("Most Expensive: {}\n", most_expensive));
        }

        if let Some(ref most_called) = report.execution_stats.most_called_function {
            info.push_str(&format!("Most Called: {}\n", most_called));
        }

        info
    }

    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Default for DebugPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// 调试标签页
#[derive(Debug, Clone, Copy)]
enum DebugTab {
    CallStack,
    Variables,
    Performance,
    Memory,
}
