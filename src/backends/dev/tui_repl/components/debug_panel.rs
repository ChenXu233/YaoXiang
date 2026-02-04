use std::sync::{Arc, Mutex};
/// 调试面板组件
///
/// 显示调试信息，包括变量和性能统计
use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::backends::dev::repl::backend_trait::REPLBackend;
use crate::backends::dev::repl::engine::Evaluator;

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
            selected_tab: DebugTab::Variables,
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
            DebugTab::Variables => DebugTab::Performance,
            DebugTab::Performance => DebugTab::Variables,
        };
    }

    /// 渲染调试面板
    pub fn render(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        evaluator: &Arc<Mutex<Evaluator>>,
    ) {
        if !self.visible {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Magenta))
            .title(Span::styled(
                " Debug ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        // 获取调试信息
        let debug_info = self.get_debug_info(evaluator);

        let paragraph = Paragraph::new(debug_info)
            .style(Style::default().fg(Color::LightGreen))
            .scroll((0, 0));

        f.render_widget(paragraph, inner_area);
    }

    /// 获取调试信息
    fn get_debug_info(
        &self,
        evaluator: &Arc<Mutex<Evaluator>>,
    ) -> String {
        let eval = evaluator.lock().unwrap();
        let stats = eval.stats();
        let symbols = eval.get_symbols();

        let mut info = String::new();

        match self.selected_tab {
            DebugTab::Variables => {
                info.push_str("[Variables]\n");
                if symbols.is_empty() {
                    info.push_str("(no variables defined)\n");
                } else {
                    for sym in symbols {
                        info.push_str(&format!("{}: {}\n", sym.name, sym.type_signature));
                    }
                }
            }
            DebugTab::Performance => {
                info.push_str("[Performance]\n");
                info.push_str(&format!("Eval count: {}\n", stats.eval_count));
                info.push_str(&format!("Total time: {:?}\n", stats.total_time));
            }
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
    Variables,
    Performance,
}
