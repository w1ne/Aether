use eframe::egui;
use egui_dock::TabViewer;
use crate::AetherApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DebugTab {
    // Left Panel Tabs
    Control,

    // Original Tabs
    Peripherals,
    Rtt,
    Source,
    Plot,
    Tasks,
    Stack,
    Timeline,
    Variables,
    Agent,

    // New Separate Tabs
    Memory,
    Disassembly,
    Logs,
}

pub struct AetherTabViewer<'a> {
    pub app: &'a mut AetherApp,
}

impl<'a> TabViewer for AetherTabViewer<'a> {
    type Tab = DebugTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            DebugTab::Control => "⚡ Control".into(),
            DebugTab::Peripherals => "☰ Peripherals".into(),
            DebugTab::Rtt => "⫘ RTT".into(),
            DebugTab::Source => "✍ Source".into(),
            DebugTab::Plot => "📈 Plot".into(),
            DebugTab::Tasks => "⚙ Tasks".into(),
            DebugTab::Stack => "⛃ Stack".into(),
            DebugTab::Timeline => "⏱ Timeline".into(),
            DebugTab::Variables => "🔎 Watch".into(),
            DebugTab::Agent => "🤖 Agent".into(),
            DebugTab::Memory => "🖴 Memory".into(),
            DebugTab::Disassembly => "☷ Disassembly".into(),
            DebugTab::Logs => "📑 Logs".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            DebugTab::Control => self.app.draw_control_view(ui),
            DebugTab::Peripherals => self.app.draw_peripherals_view(ui),
            DebugTab::Rtt => self.app.draw_rtt_view(ui),
            DebugTab::Source => self.app.draw_source_view(ui),
            DebugTab::Plot => self.app.draw_plot_view(ui),
            DebugTab::Tasks => self.app.draw_tasks_view(ui),
            DebugTab::Stack => self.app.draw_stack_view(ui),
            DebugTab::Timeline => self.app.draw_timeline_view(ui),
            DebugTab::Variables => self.app.draw_variables_view(ui),
            DebugTab::Agent => self.app.draw_agent_view(ui),
            DebugTab::Memory => self.app.draw_memory_view(ui),
            DebugTab::Disassembly => self.app.draw_disassembly_view(ui),
            DebugTab::Logs => self.app.draw_logs_view(ui),
        }
    }
}
