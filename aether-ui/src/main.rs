mod ui_logic;
use crossbeam_channel::{unbounded, Receiver};
use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::easy::HighlightLines;
use std::sync::mpsc;
use std::sync::Arc;
use aether_core::VarType;
use serde::{Serialize, Deserialize};
use tokio_stream::StreamExt;
use egui_dock::{DockArea, DockState, NodeIndex, Style};

mod ui_tabs;
use ui_tabs::{DebugTab, AetherTabViewer};

fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Aether Debugger",
        native_options,
        Box::new(|cc| Ok(Box::new(AetherApp::new(cc)))),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum RttDisplayMode {
    Text,
    Hex,
    Binary,
}

pub struct AetherApp {
    probe_manager: aether_core::ProbeManager,
    probes: Vec<aether_core::ProbeInfo>,
    selected_probe: Option<usize>,
    target_info: Option<aether_core::TargetInfo>,
    connection_status: ConnectionStatus,
    status_message: String,

    // Session & Debug state
    session_handle: Option<Arc<aether_core::SessionHandle>>,
    event_receiver: Option<tokio::sync::broadcast::Receiver<aether_core::DebugEvent>>,
    registers: HashMap<u16, u64>,
    core_status: Option<probe_rs::CoreStatus>,
    failed_requests: Vec<String>,

    // Memory state
    memory_data: Vec<u8>,
    memory_address_input: String,
    memory_base_address: u64,

    // Disassembly state
    disassembly: Vec<aether_core::disasm::InstructionInfo>,

    // Breakpoints state
    breakpoints: Vec<u64>,
    breakpoint_address_input: String,

    // Flashing state
    selected_file: Option<PathBuf>,
    flashing_progress: Option<f32>,
    flashing_status: String,
    progress_receiver: Option<Receiver<aether_core::FlashingProgress>>,

    // SVD / Peripherals state
    peripherals: Vec<aether_core::svd::PeripheralInfo>,
    selected_peripheral: Option<String>,
    peripheral_registers: Vec<aether_core::svd::RegisterInfo>,
    expanded_registers: std::collections::HashSet<String>,

    // RTT State
    rtt_attached: bool,
    rtt_up_channels: Vec<aether_core::rtt::RttChannelInfo>,
    rtt_down_channels: Vec<aether_core::rtt::RttChannelInfo>,
    rtt_selected_channel: Option<usize>,
    rtt_display_modes: HashMap<usize, RttDisplayMode>,
    rtt_buffers: std::collections::HashMap<usize, String>,
    rtt_raw_buffers: std::collections::HashMap<usize, Vec<u8>>,
    rtt_input: String,
    

    // Symbols & Source state
    symbols_loaded: bool,
    source_info: Option<aether_core::SourceInfo>,
    breakpoint_locations: Vec<aether_core::SourceInfo>,
    // Cache stores raw lines and the pre-calculated layout job for syntax highlighting
    source_cache: HashMap<PathBuf, (Vec<String>, Vec<egui::text::LayoutJob>)>,
    
    // Plot State
    plots: HashMap<String, std::collections::VecDeque<[f64; 2]>>,
    plot_names: Vec<String>,
    new_plot_name: String,
    new_plot_type: VarType,
    
    // Remote Connection State
    remote_host: String,
    remote_port: String,
    is_remote: bool,
    
    // RTOS State
    tasks: Vec<aether_core::TaskInfo>,
    timeline_events: Vec<TimelineEvent>,

    // Stack State
    stack_frames: Vec<aether_core::StackFrame>,

    // Watch State
    watched_variables: Vec<aether_core::symbols::TypeInfo>,
    variable_input: String,
    
    // Syntax Highlighting
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    
    // Docking State
    dock_state: Option<DockState<DebugTab>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub task_handle: u32,
    pub task_name: String,
    pub start_time: f64,
    pub end_time: Option<f64>,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl AetherApp {
    fn setup_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        // 1. Try to load Noto Color Emoji or DejaVu Sans for better Unicode coverage on Linux
        let font_paths = [
            "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ];

        for path in font_paths {
            if let Ok(font_data) = std::fs::read(path) {
                let font_name = path.split('/').last().unwrap_or("fallback_font").to_string();
                fonts.font_data.insert(
                    font_name.clone(),
                    egui::FontData::from_owned(font_data),
                );

                // Add to standard families
                if let Some(vec) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                    vec.push(font_name.clone());
                }
                if let Some(vec) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                    vec.push(font_name);
                }
                log::info!("Loaded fallback font: {}", path);
                break; // Stop after first successful load
            }
        }

        ctx.set_fonts(fonts);
    }

    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::setup_fonts(&cc.egui_ctx);
        Self {
            probe_manager: aether_core::ProbeManager::new(),
            probes: Vec::new(),
            selected_probe: None,
            target_info: None,
            connection_status: ConnectionStatus::Disconnected,
            status_message: "Ready".to_string(),
            session_handle: None,
            event_receiver: None,
            registers: HashMap::new(),
            core_status: None,
            failed_requests: Vec::new(),
            memory_data: Vec::new(),
            memory_address_input: "0x20000000".to_string(),
            memory_base_address: 0x20000000,
            disassembly: Vec::new(),
            breakpoints: Vec::new(),
            breakpoint_address_input: "0x08000000".to_string(),
            selected_file: None,
            flashing_progress: None,
            flashing_status: String::new(),
            progress_receiver: None,
            peripherals: Vec::new(),
            selected_peripheral: None,
            peripheral_registers: Vec::new(),
            expanded_registers: std::collections::HashSet::new(),
            rtt_attached: false,
            rtt_up_channels: Vec::new(),
            rtt_down_channels: Vec::new(),
            rtt_selected_channel: None,
            rtt_display_modes: std::collections::HashMap::new(),
            rtt_buffers: std::collections::HashMap::new(),
            rtt_raw_buffers: std::collections::HashMap::new(),
            rtt_input: String::new(),
            symbols_loaded: false,
            source_info: None,
            breakpoint_locations: Vec::new(),
            source_cache: HashMap::new(),
            // Plot State
            plots: HashMap::new(),
            plot_names: Vec::new(),
            new_plot_name: String::new(),
            new_plot_type: VarType::U32,
            remote_host: "localhost".to_string(),
            remote_port: "50051".to_string(),
            is_remote: false,
            tasks: Vec::new(),
            timeline_events: Vec::new(),
            stack_frames: Vec::new(),
            watched_variables: Vec::new(),
            variable_input: String::new(),
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            dock_state: Some({
                let mut dock_state = DockState::new(vec![DebugTab::Control]);
                let surface = dock_state.main_surface_mut();

                // 1. Split root (Control) to the right for the rest of the app
                let [_control, right_section] = surface.split_right(
                    NodeIndex::root(),
                    0.20,
                    vec![DebugTab::Peripherals, DebugTab::Variables, DebugTab::Stack, DebugTab::Tasks, DebugTab::Timeline, DebugTab::Rtt, DebugTab::Agent]
                );

                // 2. Split the right section further to the right for Source
                let [middle_section, _source] = surface.split_right(
                    right_section,
                    0.65,
                    vec![DebugTab::Source]
                );

                // 3. Split the middle section below for Memory/Logs
                let [_top_middle, _bottom_middle] = surface.split_below(
                    middle_section,
                    0.65,
                    vec![DebugTab::Memory, DebugTab::Disassembly, DebugTab::Logs]
                );

                dock_state
            }),
        }
    }

    fn refresh_probes(&mut self) {
        match self.probe_manager.list_probes() {
            Ok(probes) => {
                self.probes = probes;
                self.status_message = format!("Found {} probe(s)", self.probes.len());
            }
            Err(e) => {
                self.status_message = format!("Error listing probes: {}", e);
                self.connection_status = ConnectionStatus::Error;
            }
        }
    }

    fn connect_remote(&mut self) {
        let host = self.remote_host.clone();
        let port = self.remote_port.parse::<u16>().unwrap_or(50051);
        self.connection_status = ConnectionStatus::Connecting;
        self.is_remote = true;
        self.status_message = format!("Connecting to remote agent at {}:{}...", host, port);

        let (evt_tx, evt_rx) = tokio::sync::broadcast::channel(1024);
        self.event_receiver = Some(evt_rx);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            
            rt.block_on(async {
                let endpoint = format!("http://{}:{}", host, port);
                match aether_agent_api::proto::aether_debug_client::AetherDebugClient::connect(endpoint).await {
                    Ok(mut client) => {
                        // Success!
                        if let Ok(response) = client.subscribe_events(aether_agent_api::proto::Empty {}).await {
                             let mut stream = response.into_inner();
                             while let Some(Ok(proto_event)) = stream.next().await {
                                  if let Some(core_event) = aether_agent_api::map_proto_event_to_core(proto_event) {
                                       let _ = evt_tx.send(core_event);
                                  }
                             }
                        }
                    }
                    Err(e) => {
                        log::error!("Remote connection failed: {}", e);
                    }
                }
            });
        });
    }

    fn connect_probe(&mut self) {
        if let Some(index) = self.selected_probe {
            self.connection_status = ConnectionStatus::Connecting;
            match self.probe_manager.open_probe(index) {
                Ok(probe) => {
                    self.status_message =
                        format!("Connected to {}. Detecting target...", self.probes[index].name());
                    
                    // Detect target first - consumes probe, returns (info, session)
                    match self.probe_manager.detect_target(probe, "any", false) {
                        Ok((target, session)) => {
                            self.target_info = Some(target.clone());
                            self.status_message = format!(
                                "Connected to {} -> {}",
                                self.probes[index].name(),
                                target.name
                            );

                            // Create SessionHandle which consumes the session
                            match aether_core::SessionHandle::new(Some(session)) {
                                Ok(handle) => {
                                     let handle = Arc::new(handle);
                                     self.event_receiver = Some(handle.subscribe());
                                     self.session_handle = Some(handle.clone());
                                     self.connection_status = ConnectionStatus::Connected;
                                     
                                     // Spawn Agent API Server
                                     let server_handle = handle.clone();
                                     std::thread::spawn(move || {
                                         let rt = tokio::runtime::Builder::new_current_thread()
                                             .enable_all()
                                             .build()
                                             .unwrap();
                                         
                                         rt.block_on(async {
                                             if let Err(e) = aether_agent_api::run_server(server_handle, "0.0.0.0", 50051).await {
                                                 log::error!("Agent API Server Error: {}", e);
                                             }
                                         });
                                     });

                                     // Initial Poll
                                     if let Some(h) = &self.session_handle {
                                         let _ = h.send(aether_core::DebugCommand::PollStatus);
                                         let _ = h.send(aether_core::DebugCommand::GetTasks);
                                         // Request some registers
                                         for i in 0..16 {
                                             let _ = h.send(aether_core::DebugCommand::ReadRegister(i));
                                         }
                                         // Request initial memory
                                          let _ = h.send(aether_core::DebugCommand::ReadMemory(self.memory_base_address, 256));
                                          // Request current breakpoints
                                          let _ = h.send(aether_core::DebugCommand::ListBreakpoints);
                                     }
                                }
                                Err(e) => {
                                    self.connection_status = ConnectionStatus::Error;
                                    self.status_message = format!("Failed to create session: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            self.connection_status = ConnectionStatus::Error;
                            self.status_message = format!("Failed to detect target: {}", e);
                        }
                    }
                }
                Err(e) => {
                    self.connection_status = ConnectionStatus::Error;
                    self.status_message = format!("Failed to connect: {}", e);
                }
            }
        }
    }

    fn start_flashing(&mut self) {
        let file_path = if let Some(path) = &self.selected_file {
            path.clone()
        } else {
            return;
        };

        if self.connection_status != ConnectionStatus::Connected {
            self.status_message = "Connect to a probe first!".to_string();
            return;
        }
        
        // NOTE: Flashing currently re-opens the probe in a separate thread.
        // This conflicts with SessionHandle which OWNS the probe.
        // For Milestone 3, we should ideally integrate flashing into SessionHandle,
        // OR drop SessionHandle temporarily.
        // For now, let's warn user or implement dropping.
        
        // Drop existing session to release probe
        self.session_handle = None;
        self.connection_status = ConnectionStatus::Disconnected; // Will reconnect after?
        
        // ... Flashing Logic (Same as before but we lost connection) ...
        // Real implementation would pass Session to FlashManager.
        // But FlashManager takes `&mut Session`. SessionHandle runs in thread.
        // Complex. For now, let's keep the existing "Re-open" logic but we MUST ensure the probe is free.
        // Dropping session_handle frees the probe (thread finishes).
        
        // Wait a bit for thread to backend drop?
        // Let's just proceed with existing flashing logic, but user has to reconnect.

        let (tx, rx) = unbounded();
        self.progress_receiver = Some(rx);
        self.flashing_progress = Some(0.0);
        self.flashing_status = "Preparing to flash...".to_string();

        let probe_index = self.selected_probe.unwrap();
        let flash_manager = aether_core::FlashManager::new();

        std::thread::spawn(move || {
            // Need a slight delay to ensure previous session dropped?
            std::thread::sleep(std::time::Duration::from_millis(100));
            
            let probe_manager = aether_core::ProbeManager::new();
            match probe_manager.open_probe(probe_index) {
                Ok(probe) => {
                    match probe.attach("any", probe_rs::Permissions::default()) {
                        Ok(mut session) => {
                            let (mpsc_tx, mpsc_rx) = mpsc::channel();
                            let progress =
                                aether_core::MpscFlashProgress::new(mpsc_tx).into_flash_progress();

                            let tx_clone = tx.clone();
                            std::thread::spawn(move || {
                                while let Ok(p) = mpsc_rx.recv() {
                                    let _ = tx_clone.send(p);
                                }
                            });

                            if let Err(e) =
                                flash_manager.flash_elf(&mut session, &file_path, progress)
                            {
                                let _ = tx.send(aether_core::FlashingProgress::Failed);
                                log::error!("Flashing failed: {}", e);
                            } else {
                                // Flashing done
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(aether_core::FlashingProgress::Failed);
                            log::error!("Failed to attach: {}", e);
                        }
                    }
                }
                Err(_) => {
                    let _ = tx.send(aether_core::FlashingProgress::Failed);
                }
            }
        });
    }

    fn update_flashing(&mut self) {
        let mut finished = false;
        let mut failed = false;

        if let Some(rx) = &self.progress_receiver {
            while let Ok(progress) = rx.try_recv() {
                match progress {
                    aether_core::FlashingProgress::Started => {
                        self.flashing_status = "Starting...".to_string();
                    }
                    aether_core::FlashingProgress::Erasing => {
                        self.flashing_status = "Erasing...".to_string();
                        self.flashing_progress = Some(0.1);
                    }
                    aether_core::FlashingProgress::Programming { total } => {
                        self.flashing_status = format!("Programming {} bytes...", total);
                        self.flashing_progress = Some(0.3);
                    }
                    aether_core::FlashingProgress::Progress { bytes } => {
                        if let Some(p) = self.flashing_progress {
                            self.flashing_progress = Some((p + 0.05).min(0.95));
                        }
                        self.flashing_status = format!("Progress: {} bytes", bytes);
                    }
                    aether_core::FlashingProgress::Finished => {
                        self.flashing_status = "Done!".to_string();
                        self.flashing_progress = Some(1.0);
                        finished = true;
                    }
                    aether_core::FlashingProgress::Failed => {
                        self.flashing_status = "Failed!".to_string();
                        self.flashing_progress = None;
                        failed = true;
                    }
                    aether_core::FlashingProgress::Message(m) => {
                        self.flashing_status = m;
                    }
                    _ => {}
                }
            }
        }

        if finished || failed {
            self.progress_receiver = None;
        }
    }
    
    fn process_debug_events(&mut self) {
        let handle = if let Some(h) = &self.session_handle {
            h.clone()
        } else {
            return;
        };

        let mut events = Vec::new();
        if let Some(rx) = &mut self.event_receiver {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }

        for event in events {
            match event {
                    aether_core::DebugEvent::Status(status) => {
                        self.core_status = Some(status);
                    }
                    aether_core::DebugEvent::Halted { pc } => {
                       self.status_message = format!("Halted at PC=0x{:08X}", pc);
                       // Update status
                       let _ = handle.send(aether_core::DebugCommand::PollStatus);
                       // Update registers
                        for i in 0..16 {
                             let _ = handle.send(aether_core::DebugCommand::ReadRegister(i));
                         }
                         // Update memory
                         let _ = handle.send(aether_core::DebugCommand::ReadMemory(self.memory_base_address, 256));
                         // Request disassembly
                         let _ = handle.send(aether_core::DebugCommand::Disassemble(pc, 64)); // 32 instructions roughly
                         // Request source info
                         let _ = handle.send(aether_core::DebugCommand::LookupSource(pc));
                         // Request stack
                         let _ = handle.send(aether_core::DebugCommand::GetStack);
                    }
                    aether_core::DebugEvent::Resumed => {
                        self.status_message = "Running...".to_string();
                        // Update status
                       let _ = handle.send(aether_core::DebugCommand::PollStatus);
                    }
                    aether_core::DebugEvent::RegisterValue(address, value) => {
                        self.registers.insert(address, value);
                    }
                    aether_core::DebugEvent::MemoryData(address, data) => {
                        if address == self.memory_base_address {
                            self.memory_data = data;
                        }
                    }
                    aether_core::DebugEvent::Disassembly(insns) => {
                        self.disassembly = insns;
                    }
                    aether_core::DebugEvent::Breakpoints(bps) => {
                        self.breakpoints = bps;
                    }
                    aether_core::DebugEvent::SvdLoaded => {
                        self.status_message = "SVD Loaded".to_string();
                        if let Some(handle) = &self.session_handle {
                            let _ = handle.send(aether_core::DebugCommand::GetPeripherals);
                        }
                    }
                    aether_core::DebugEvent::Peripherals(periphs) => {
                        self.selected_peripheral = None;
                        self.peripherals = periphs;
                    }
                    aether_core::DebugEvent::RttChannels { up_channels, down_channels } => {
                        self.rtt_attached = true;
                        self.rtt_up_channels = up_channels;
                        self.rtt_down_channels = down_channels;
                        if self.rtt_selected_channel.is_none() && !self.rtt_up_channels.is_empty() {
                            self.rtt_selected_channel = Some(self.rtt_up_channels[0].number);
                        }
                    }
                    aether_core::DebugEvent::RttData(channel, data) => {
                        // Store raw bytes for Hex/Binary views
                        let raw_buf = self.rtt_raw_buffers.entry(channel).or_default();
                        raw_buf.extend_from_slice(&data);
                        if raw_buf.len() > 65536 {
                            let truncate_at = raw_buf.len() - 65536;
                            raw_buf.drain(0..truncate_at);
                        }

                        let text = String::from_utf8_lossy(&data).to_string();
                        self.rtt_buffers.entry(channel).or_default().push_str(&text);
                        // Limit buffer size to 64KB for performance
                        if self.rtt_buffers.get(&channel).map_or(0, |s| s.len()) > 65536 {
                            let buf = self.rtt_buffers.get_mut(&channel).unwrap();
                            let truncate_at = buf.len() - 65536;
                            *buf = buf[truncate_at..].to_string();
                        }
                    }
                    aether_core::DebugEvent::PlotData { name, timestamp, value } => {
                        let deque = self.plots.entry(name.clone()).or_insert_with(std::collections::VecDeque::new);
                        deque.push_back([timestamp, value]);
                        if deque.len() > 100_000 {
                            deque.pop_front();
                        }
                        if !self.plot_names.contains(&name) {
                            self.plot_names.push(name.clone());
                        }
                    }
                    aether_core::DebugEvent::Tasks(tasks) => {
                        self.tasks = tasks;
                    }
                    aether_core::DebugEvent::TaskSwitch { from, to, timestamp } => {
                        // 1. Close previous task if it exists
                        if let Some(from_handle) = from {
                            if let Some(event) = self.timeline_events.iter_mut().rev().find(|e| e.task_handle == from_handle && e.end_time.is_none()) {
                                event.end_time = Some(timestamp);
                            }
                        }
                        
                        // 2. Open new task
                        let name = self.tasks.iter().find(|t| t.handle == to).map(|t| t.name.clone()).unwrap_or_else(|| format!("0x{:08X}", to));
                        self.timeline_events.push(TimelineEvent {
                            task_handle: to,
                            task_name: name,
                            start_time: timestamp,
                            end_time: None,
                        });
                        
                        // Prune history (keep last 500 events for performance)
                        if self.timeline_events.len() > 500 {
                            self.timeline_events.remove(0);
                        }
                    }
                    aether_core::DebugEvent::Stack(frames) => {
                        self.stack_frames = frames;
                    }
                    aether_core::DebugEvent::Registers(regs) => {
                        self.peripheral_registers = regs;
                    }
                    aether_core::DebugEvent::SymbolsLoaded => {
                        self.symbols_loaded = true;
                        self.status_message = "Symbols Loaded".to_string();
                    }
                    aether_core::DebugEvent::SourceLocation(info) => {
                        // Load source file if not in cache
                        if !self.source_cache.contains_key(&info.file) {
                            if let Ok(content) = std::fs::read_to_string(&info.file) {
                                let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                                let highlighted = self.highlight_file(&info.file, &content);
                                self.source_cache.insert(info.file.clone(), (lines, highlighted));
                            }
                        }
                        self.source_info = Some(info);
                        // TODO: Focus Source tab in DockState (requires iterating/finding tab)
                        // if let Some(dock_state) = &mut self.dock_state { ... }
                    }
                    aether_core::DebugEvent::BreakpointLocations(locs) => {
                        self.breakpoint_locations = locs;
                    }
                    aether_core::DebugEvent::VariableResolved(info) => {
                        // If variable already in watch list, update it, otherwise add it
                        if let Some(pos) = self.watched_variables.iter().position(|v| v.name == info.name) {
                            self.watched_variables[pos] = info;
                        } else {
                            self.watched_variables.push(info);
                        }
                    }
                    aether_core::DebugEvent::Error(e) => {
                         self.failed_requests.push(e.clone());
                         log::error!("Debug Error: {}", e);
                    }
                    aether_core::DebugEvent::TraceData(_data) => {
                        // Handle trace data (placeholder for visualization)
                    }
                    aether_core::DebugEvent::FlashProgress(p) => {
                        self.flashing_progress = Some(p);
                    }
                    aether_core::DebugEvent::FlashStatus(s) => {
                        self.flashing_status = s;
                    }
                    aether_core::DebugEvent::FlashDone => {
                        self.flashing_progress = Some(1.0);
                        self.flashing_status = "Flashing Successful".to_string();
                    }
                    aether_core::DebugEvent::SemihostingOutput(msg) => {
                        self.status_message = format!("Semihosting: {}", msg);
                    }
                    aether_core::DebugEvent::ItmPacket(_) => {
                        // ITM Visualization pending
                    }
                    aether_core::DebugEvent::Probes(_) | aether_core::DebugEvent::Attached(_) => {}
                }
            }
    }

    pub(crate) fn draw_agent_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("🤖 Agent Interface");
        ui.add_space(8.0);
        
        // 1. Status Section
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Status:").strong());
                if self.session_handle.is_some() {
                     ui.label(egui::RichText::new("Active (Listening on 0.0.0.0:50051)").color(egui::Color32::GREEN));
                } else {
                     ui.label(egui::RichText::new("Inactive (Connect Probe First)").color(egui::Color32::RED));
                }
            });
            ui.add_space(4.0);
            ui.label("The Agent API allows AI agents to control the debugger via gRPC.");
        });

        ui.add_space(8.0);
        ui.separator();

        // 2. System Logs (Collapsible)
        ui.collapsing("📝 System Logs & Quick Connect", |ui| {
             ui.heading("System Logs");
             egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                 ui.monospace(&self.status_message);
                 for req in &self.failed_requests {
                      ui.label(egui::RichText::new(format!("Error: {}", req)).color(egui::Color32::RED));
                 }
             });
             
             ui.add_space(8.0);
             ui.heading("Quick Connect (Python)");
             let code = r#"import grpc
import aether_pb2
import aether_pb2_grpc

channel = grpc.insecure_channel('localhost:50051')
stub = aether_pb2_grpc.AetherDebugStub(channel)

# Halt and Inspect (Synchronous)
stub.Halt(aether_pb2.Empty())
print(stub.ReadRegister(aether_pb2.ReadRegisterRequest(register_number=15)))
stub.Resume(aether_pb2.Empty())
"#;
             let mut code_buf = code.to_string();
             ui.add(egui::TextEdit::multiline(&mut code_buf).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
        });
    }

    pub(crate) fn draw_plot_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Live Variable Plotting");

        ui.horizontal(|ui| {
             ui.label("Name/Addr:");
             ui.text_edit_singleline(&mut self.new_plot_name);
             
             egui::ComboBox::from_label("Type")
                 .selected_text(format!("{:?}", self.new_plot_type))
                 .show_ui(ui, |ui| {
                      ui.selectable_value(&mut self.new_plot_type, VarType::U32, "U32");
                      ui.selectable_value(&mut self.new_plot_type, VarType::I32, "I32");
                      ui.selectable_value(&mut self.new_plot_type, VarType::F32, "F32");
                      ui.selectable_value(&mut self.new_plot_type, VarType::U8, "U8");
                      ui.selectable_value(&mut self.new_plot_type, VarType::F64, "F64");
                 });
                 
             if ui.button("Add Plot").clicked() {
                  if !self.new_plot_name.is_empty() {
                       if let Some(handle) = &self.session_handle {
                            let _ = handle.send(aether_core::DebugCommand::AddPlot {
                                 name: self.new_plot_name.clone(),
                                 var_type: self.new_plot_type
                            });
                       }
                  }
             }
        });

        ui.separator();
        
        // Use fully qualified names to avoid import issues
        let plot = egui_plot::Plot::new("live_plot")
            .legend(egui_plot::Legend::default())
            .height(400.0);
            
        plot.show(ui, |plot_ui| {
             for name in &self.plot_names {
                  if let Some(data) = self.plots.get(name) {
                       let points: egui_plot::PlotPoints = data.iter().copied().collect();
                       plot_ui.line(egui_plot::Line::new(points).name(name));
                  }
             }
        });
        
        ui.separator();
        ui.label("Active Plots:");
        
        let mut to_remove = Vec::new();
        for name in &self.plot_names {
             ui.horizontal(|ui| {
                  ui.label(name);
                  if ui.button("Remove").clicked() {
                       to_remove.push(name.clone());
                       if let Some(handle) = &self.session_handle {
                            let _ = handle.send(aether_core::DebugCommand::RemovePlot(name.clone()));
                       }
                  }
             });
        }
        
        for name in to_remove {
             self.plots.remove(&name);
             if let Some(idx) = self.plot_names.iter().position(|x| *x == name) {
                 self.plot_names.remove(idx);
             }
        }
    }

    pub(crate) fn draw_tasks_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("🛰 RTOS Analysis");
            ui.add_space(8.0);
            if ui.button("🔄 Refresh").clicked() {
                if let Some(h) = &self.session_handle {
                    let _ = h.send(aether_core::DebugCommand::GetTasks);
                }
            }
        });
        
        ui.separator();

        // Calculate CPU usage percentages for the last 1 second
        let mut cpu_stats: HashMap<u32, f64> = HashMap::new();
        let now = self.timeline_events.iter().map(|e| e.end_time.unwrap_or(e.start_time)).fold(0.0, f64::max);
        let window = 1.0; // 1 second window
        let start_window = (now - window).max(0.0);
        
        for event in &self.timeline_events {
            let start = event.start_time.max(start_window);
            let end = event.end_time.unwrap_or(now).max(start_window);
            
            if end > start {
                let duration = end - start;
                *cpu_stats.entry(event.task_handle).or_insert(0.0) += duration;
            }
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            
            egui::Grid::new("tasks_grid")
                .striped(true)
                .num_columns(7)
                .spacing([25.0, 8.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Type").strong());
                    ui.label(egui::RichText::new("Task Name").strong());
                    ui.label(egui::RichText::new("State").strong());
                    ui.label(egui::RichText::new("Prio").strong());
                    ui.label(egui::RichText::new("CPU%").strong());
                    ui.label(egui::RichText::new("Handle").strong());
                    ui.label(egui::RichText::new("Stack Usage").strong());
                    ui.end_row();

                    for task in &self.tasks {
                        let type_icon = match task.task_type {
                            aether_core::TaskType::Thread => "🧵",
                            aether_core::TaskType::Async => "⚡",
                        };
                        ui.label(type_icon);

                        ui.label(&task.name);
                        
                        let state_text = ui_logic::get_task_state_display(task.state);
                        let state_color = match task.state {
                             aether_core::TaskState::Running => egui::Color32::from_rgb(0, 255, 0),
                             aether_core::TaskState::Ready => egui::Color32::from_rgb(0, 200, 255),
                             _ => egui::Color32::GRAY,
                        };
                        ui.label(egui::RichText::new(state_text).color(state_color));
                        
                        ui.label(task.priority.to_string());
                        
                        // CPU Usage %
                        let cpu_usage = cpu_stats.get(&task.handle).cloned().unwrap_or(0.0) / window * 100.0;
                        ui.label(format!("{:.1}%", cpu_usage.min(100.0)));
                        
                        ui.monospace(format!("0x{:08X}", task.handle));
                        
                        // Stack Usage Bar
                        let stack_fraction = if task.stack_size > 0 {
                            task.stack_usage as f32 / task.stack_size as f32
                        } else {
                            0.0
                        };
                        
                        ui.horizontal(|ui| {
                            let bar_color = if stack_fraction > 0.9 {
                                egui::Color32::RED
                            } else if stack_fraction > 0.7 {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::from_rgb(0, 255, 150)
                            };
                            
                            ui.add(egui::ProgressBar::new(stack_fraction)
                                .text(format!("{} / {}", task.stack_usage, task.stack_size))
                                .fill(bar_color)
                                .desired_width(120.0));
                        });
                        
                        ui.end_row();
                    }
                });
            
            if self.tasks.is_empty() {
                ui.add_space(20.0);
                ui.label(egui::RichText::new("No RTOS tasks detected.").italics().color(egui::Color32::GRAY));
                ui.label("Ensure FreeRTOS is running and symbols are correctly loaded.");
            }
        });
    }

    pub(crate) fn draw_timeline_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("📊 Execution Timeline");
            ui.add_space(8.0);
            if ui.button("🗑 Clear").clicked() {
                self.timeline_events.clear();
            }
        });
        
        ui.separator();

        // Prepare task mappings for labels and coloring
        let mut sorted_handles: Vec<u32> = self.tasks.iter().map(|t| t.handle).collect();
        for event in &self.timeline_events {
            if !sorted_handles.contains(&event.task_handle) {
                sorted_handles.push(event.task_handle);
            }
        }
        sorted_handles.sort();

        let mut task_slots: HashMap<u32, f64> = HashMap::new();
        let mut slot_to_name: HashMap<i32, String> = HashMap::new();
        for (i, handle) in sorted_handles.iter().enumerate() {
            task_slots.insert(*handle, i as f64);
            let name = self.tasks.iter().find(|t| t.handle == *handle).map(|t| t.name.clone()).unwrap_or_else(|| format!("0x{:08X}", handle));
            slot_to_name.insert(i as i32, name);
        }

        let plot = egui_plot::Plot::new("timeline_plot")
            .height(400.0)
            .show_x(true)
            .show_y(true)
            .y_axis_formatter(move |val, _range| {
                slot_to_name.get(&(val.value.round() as i32)).cloned().unwrap_or_default()
            })
            .label_formatter(|name, value| {
                if name.is_empty() {
                    format!("Time: {:.3}s", value.x)
                } else {
                    format!("Task: {}\nTime: {:.3}s", name, value.x)
                }
            })
            .allow_zoom([true, false]) // Zoom mostly on X axis for timeline
            .allow_drag(true);

        plot.show(ui, |plot_ui| {
            for event in &self.timeline_events {
                if let Some(&slot) = task_slots.get(&event.task_handle) {
                    let start = event.start_time;
                    let end = event.end_time.unwrap_or_else(|| {
                        // If it's the latest event, show it ending at "now" (max start + buffer)
                        self.timeline_events.iter().map(|e| e.end_time.unwrap_or(e.start_time)).fold(start, f64::max) + 0.01
                    });

                    let rect = egui_plot::PlotPoints::from_iter(vec![
                        [start, slot - 0.35],
                        [end, slot - 0.35],
                        [end, slot + 0.35],
                        [start, slot + 0.35],
                        [start, slot - 0.35],
                    ]);
                    
                    let color = egui::Color32::from_rgb(
                        ((event.task_handle >> 16) & 0xFF) as u8,
                        ((event.task_handle >> 8) & 0xFF) as u8,
                        (event.task_handle & 0xFF) as u8,
                    ).gamma_multiply(0.8);

                    plot_ui.polygon(egui_plot::Polygon::new(rect)
                        .fill_color(color)
                        .name(format!("{} (Duration: {:.1}ms)", event.task_name, (end - start) * 1000.0)));
                }
            }
        });

        ui.add_space(4.0);
        ui.label(egui::RichText::new("💡 Hint: Use mouse wheel to zoom X-axis. Click and drag to pan.").small().color(egui::Color32::GRAY));
        ui.label("Vertical axis shows different RTOS tasks. Horizontal axis is session time (s).");
    }

    pub(crate) fn draw_stack_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Call Stack");
        
        if ui.button("🔄 Refresh Stack").clicked() {
            if let Some(h) = &self.session_handle {
                let _ = h.send(aether_core::DebugCommand::GetStack);
            }
        }
        ui.separator();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
             egui::Grid::new("stack_grid").striped(true).show(ui, |ui| {
                 ui.label("#");
                 ui.label("Function");
                 ui.label("Location");
                 ui.label("PC");
                 ui.end_row();
                 
                 for (i, frame) in self.stack_frames.iter().enumerate() {
                     ui.label(format!("{}", i));
                     ui.label(&frame.function_name);
                     
                     let loc_text = ui_logic::get_display_location(frame.source_file.as_deref(), frame.line);
                     
                     if ui.link(loc_text).clicked() {
                         if let (Some(file), Some(line)) = (&frame.source_file, frame.line) {
                             let info = aether_core::SourceInfo {
                                 file: std::path::PathBuf::from(file),
                                 line: line as u32,
                                 function: Some(frame.function_name.clone()),
                                 column: Some(0),
                             };
                             
                             if !self.source_cache.contains_key(&info.file) {
                                if let Ok(content) = std::fs::read_to_string(&info.file) {
                                    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                                    let highlighted = self.highlight_file(&info.file, &content);
                                    self.source_cache.insert(info.file.clone(), (lines, highlighted));
                                }
                             }
                             self.source_info = Some(info);
                             // TODO: Focus Source tab in DockState (requires iterating/finding tab)
                        // if let Some(dock_state) = &mut self.dock_state { ... }
                         }
                     }
                     
                     ui.monospace(format!("0x{:08X}", frame.pc));
                     ui.end_row();
                 }
             });
        });
    }


    pub(crate) fn draw_memory_view(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::both().id_source("mem_view_scroll").show(ui, |ui| {
            ui.heading("Memory View");
            
            ui.horizontal(|ui| {
                 ui.label("Addr:");
                 if ui.text_edit_singleline(&mut self.memory_address_input).lost_focus() {
                     let addr_str = self.memory_address_input.trim_start_matches("0x");
                     if let Ok(addr) = u64::from_str_radix(addr_str, 16) {
                         self.memory_base_address = addr;
                         if let Some(handle) = &self.session_handle {
                             let _ = handle.send(aether_core::DebugCommand::ReadMemory(addr, 256));
                         }
                     }
                 }
            });
             
             if ui.button("Read").clicked() {
                 let addr_str = self.memory_address_input.trim_start_matches("0x");
                 if let Ok(addr) = u64::from_str_radix(addr_str, 16) {
                     self.memory_base_address = addr;
                     if let Some(handle) = &self.session_handle {
                         let _ = handle.send(aether_core::DebugCommand::ReadMemory(addr, 256));
                     }
                 }
             }
        });
        
        egui::ScrollArea::vertical().id_source("mem_hex").show(ui, |ui| {
             ui.monospace("Address    00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F  ASCII");
             ui.separator();
             
             let bytes_per_line = 16;
             for (i, chunk) in self.memory_data.chunks(bytes_per_line).enumerate() {
                 let addr = self.memory_base_address + (i * bytes_per_line) as u64;
                 
                 let (addr_str, hex_part, ascii_part) = ui_logic::format_memory_line(addr, chunk);
                 ui.monospace(format!("{}   {} {}", addr_str, hex_part, ascii_part));
             }
        });
    }
    pub(crate) fn draw_disassembly_view(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::both().id_source("disasm_view_scroll").show(ui, |ui| {
            ui.heading("Disassembly");
            
            egui::Grid::new("disasm_grid")
                .striped(true)
                .num_columns(5)
                .show(ui, |ui| {
                    ui.label("BP");
                    ui.label("Address");
                    ui.label("Instruction");
                    ui.label("Arguments");
                    ui.label("Action");
                    ui.end_row();

                    let pc = self.registers.get(&15).cloned().unwrap_or(0); // R15 is PC in ARM

                    for insn in &self.disassembly {
                        let is_pc = insn.address == pc;
                        let is_bp = self.breakpoints.contains(&insn.address);
                        
                        let bp_marker = if is_bp { "●" } else { "○" };
                        let marker_color = if is_bp { egui::Color32::RED } else { egui::Color32::GRAY };
                        
                        if ui.colored_label(marker_color, bp_marker).clicked() {
                             if let Some(handle) = &self.session_handle {
                                 if is_bp {
                                     let _ = handle.send(aether_core::DebugCommand::ClearBreakpoint(insn.address));
                                 } else {
                                     let _ = handle.send(aether_core::DebugCommand::SetBreakpoint(insn.address));
                                 }
                             }
                        }

                        let text_color = if is_pc { egui::Color32::YELLOW } else { egui::Color32::WHITE };
                        
                        ui.colored_label(text_color, format!("0x{:08X}", insn.address));
                        ui.colored_label(text_color, &insn.mnemonic);
                        ui.colored_label(text_color, &insn.op_str);
                        
                        if ui.button("⏩").on_hover_text("Run to here").clicked() {
                            if let Some(handle) = &self.session_handle {
                                let _ = handle.send(aether_core::DebugCommand::SetBreakpoint(insn.address));
                                let _ = handle.send(aether_core::DebugCommand::Resume);
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    }

    pub(crate) fn draw_breakpoints_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Breakpoints");
        
        ui.horizontal(|ui| {
            ui.label("Addr:");
            ui.text_edit_singleline(&mut self.breakpoint_address_input);
            if ui.button("Add").clicked() {
                let addr_str = self.breakpoint_address_input.trim_start_matches("0x");
                if let Ok(addr) = u64::from_str_radix(addr_str, 16) {
                    if let Some(handle) = &self.session_handle {
                        let _ = handle.send(aether_core::DebugCommand::SetBreakpoint(addr));
                    }
                }
            }
        });
        
        ui.separator();
        
        egui::ScrollArea::vertical().id_source("bps").max_height(200.0).show(ui, |ui| {
            egui::Grid::new("bp_grid").striped(true).show(ui, |ui| {
                ui.label("Address");
                ui.label("Action");
                ui.end_row();
                
                for &addr in &self.breakpoints {
                    ui.label(format!("0x{:08X}", addr));
                    if ui.button("Remove").clicked() {
                        if let Some(handle) = &self.session_handle {
                            let _ = handle.send(aether_core::DebugCommand::ClearBreakpoint(addr));
                        }
                    }
                    ui.end_row();
                }
            });
        });
    }

    pub(crate) fn draw_peripherals_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Peripherals (SVD)");
        
        ui.horizontal(|ui| {
             if ui.button("📂 Load SVD").clicked() {
                  if let Some(path) = rfd::FileDialog::new()
                      .add_filter("SVD", &["svd"])
                      .pick_file() 
                  {
                      if let Some(handle) = &self.session_handle {
                          let _ = handle.send(aether_core::DebugCommand::LoadSvd(path));
                      }
                  }
             }
             if ui.button("🔄 Refresh").clicked() {
                  if let Some(handle) = &self.session_handle {
                       let _ = handle.send(aether_core::DebugCommand::GetPeripherals);
                       if let Some(p_name) = &self.selected_peripheral {
                           let _ = handle.send(aether_core::DebugCommand::ReadPeripheralValues(p_name.clone()));
                       }
                  }
             }
        });

        ui.separator();

        egui::ScrollArea::vertical().id_source("periph_scroll").max_height(200.0).show(ui, |ui| {
             for p in &self.peripherals {
                  let is_selected = self.selected_peripheral.as_ref() == Some(&p.name);
                  if ui.selectable_label(is_selected, &p.name).clicked() {
                       self.selected_peripheral = Some(p.name.clone());
                       if let Some(handle) = &self.session_handle {
                            let _ = handle.send(aether_core::DebugCommand::GetRegisters(p.name.clone()));
                       }
                  }
             }
        });

        ui.separator();

        if let Some(p_name) = &self.selected_peripheral {
             ui.horizontal(|ui| {
                  ui.label(format!("Peripheral: {}", p_name));
                  if ui.button("📥 Read Values").clicked() {
                       if let Some(handle) = &self.session_handle {
                            let _ = handle.send(aether_core::DebugCommand::ReadPeripheralValues(p_name.clone()));
                       }
                  }
             });
             egui::ScrollArea::vertical().id_source("reg_scroll").show(ui, |ui| {
                  for reg in &self.peripheral_registers {
                       let is_expanded = self.expanded_registers.contains(&reg.name);
                       
                       ui.horizontal(|ui| {
                            let marker = if is_expanded { "▼" } else { "▶" };
                            if ui.button(marker).clicked() {
                                 if is_expanded {
                                     self.expanded_registers.remove(&reg.name);
                                 } else {
                                     self.expanded_registers.insert(reg.name.clone());
                                 }
                            }
                            ui.label(format!("{}: +0x{:04X}", reg.name, reg.address_offset));
                            
                            if let Some(val) = reg.value {
                                 ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                     ui.monospace(format!("0x{:08X}", val));
                                 });
                            }
                       });

                       if is_expanded {
                            ui.indent("fields", |ui| {
                                 for field in &reg.fields {
                                      ui.horizontal(|ui| {
                                           ui.label(format!("{}: [{}..{}]", 
                                               field.name, 
                                               field.bit_offset, 
                                               field.bit_offset + field.bit_width - 1));
                                           
                                           if let Some(val) = reg.value {
                                                let mut field_val = field.decode(val);
                                                let field_max = (1u64 << field.bit_width) - 1;

                                                ui.label("=");
                                                if ui.add(egui::DragValue::new(&mut field_val)
                                                    .speed(1.0)
                                                    .range(0..=field_max)
                                                    .hexadecimal(field.bit_width as usize / 4 + 1, true, false)
                                                ).changed() {
                                                     if let Some(handle) = &self.session_handle {
                                                          let _ = handle.send(aether_core::DebugCommand::WritePeripheralField {
                                                               peripheral: self.selected_peripheral.as_ref().unwrap().clone(),
                                                               register: reg.name.clone(),
                                                               field: field.name.clone(),
                                                               value: field_val,
                                                          });
                                                     }
                                                }
                                           }
                                      });
                                 }
                            });
                       }
                  }
             });
        } else {
             ui.label("Select a peripheral to view registers");
        }
    }

    pub(crate) fn draw_variables_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("🔍 Variable Watch");
            ui.add_space(8.0);
            if ui.button("🔄 Refresh All").clicked() {
                if let Some(handle) = &self.session_handle {
                    for var in &self.watched_variables {
                        let _ = handle.send(aether_core::DebugCommand::WatchVariable(var.name.clone()));
                    }
                }
            }
        });
        ui.add_space(4.0);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Variable:");
                let response = ui.add(egui::TextEdit::singleline(&mut self.variable_input)
                    .hint_text("name (e.g. status_flag)")
                    .desired_width(150.0));
                
                if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || 
                   ui.button(egui::RichText::new("➕ Add").strong()).clicked() {
                     if let Some(handle) = &self.session_handle {
                          let _ = handle.send(aether_core::DebugCommand::WatchVariable(self.variable_input.clone()));
                          self.variable_input.clear();
                     }
                }
            });
        });

        ui.add_space(8.0);

        egui::ScrollArea::vertical().id_source("watch_scroll").show(ui, |ui| {
            let mut to_remove = None;
            for (idx, var) in self.watched_variables.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.push_id(idx, |ui| {
                        self.render_type_info_tree(ui, var);
                    });
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(egui::RichText::new("🗑").color(egui::Color32::GRAY)).on_hover_text("Remove from watch").clicked() {
                            to_remove = Some(idx);
                        }
                    });
                });
                ui.add_space(2.0);
                ui.separator();
            }
            if let Some(idx) = to_remove {
                self.watched_variables.remove(idx);
            }
        });
    }

    fn render_type_info_tree(&self, ui: &mut egui::Ui, info: &aether_core::symbols::TypeInfo) {
        let icon = match info.kind.as_str() {
            "Struct" => "📦",
            "Union" => "🌓",
            "Enum" => "📋",
            "Primitive" => "💎",
            "Array" => "🍱",
            "Pointer" => "🔗",
            _ => "❓",
        };

        if let Some(members) = &info.members {
            egui::collapsing_header::CollapsingHeader::new(
                egui::RichText::new(format!("{} {} ({})", icon, info.name, info.kind))
                    .strong()
                    .color(egui::Color32::from_rgb(200, 200, 200))
            ).show(ui, |ui| {
                for member in members {
                    self.render_type_info_tree(ui, member);
                }
            });
        } else {
            ui.horizontal(|ui| {
                ui.add_space(12.0); // Indent for non-collapsing items
                ui.label(egui::RichText::new(icon).small());
                ui.label(egui::RichText::new(&info.name).color(egui::Color32::from_rgb(0, 255, 255)));
                ui.label(egui::RichText::new("=").color(egui::Color32::GRAY));
                
                let val_color = if info.value_formatted_string == "Error Reading" {
                    egui::Color32::RED
                } else {
                    egui::Color32::from_rgb(0, 255, 150)
                };
                
                ui.label(egui::RichText::new(&info.value_formatted_string)
                    .monospace()
                    .color(val_color));
            });
        }
    }

    pub(crate) fn draw_rtt_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("🔌 Attach RTT").clicked() {
                if let Some(handle) = &self.session_handle {
                    let _ = handle.send(aether_core::DebugCommand::RttAttach);
                }
            }
            if self.rtt_attached {
                ui.label("✅ Attached");
            }

            ui.add_space(8.0);
            if let Some(chan_num) = self.rtt_selected_channel {
                let mode = self.rtt_display_modes.entry(chan_num).or_insert(RttDisplayMode::Text);
                ui.label("View:");
                ui.selectable_value(mode, RttDisplayMode::Text, "Text");
                ui.selectable_value(mode, RttDisplayMode::Hex, "Hex");
            }
        });

        if !self.rtt_attached {
            ui.label("RTT not attached. Click 'Attach RTT' to scan for control block.");
            return;
        }

        ui.horizontal(|ui| {
            ui.label("Channel:");
            for chan in &self.rtt_up_channels {
                let name = chan.name.as_deref().unwrap_or("unnamed");
                if ui.selectable_label(self.rtt_selected_channel == Some(chan.number), format!("{}: {}", chan.number, name)).clicked() {
                    self.rtt_selected_channel = Some(chan.number);
                }
            }
        });

        ui.separator();

        if let Some(chan_num) = self.rtt_selected_channel {
            let mode = *self.rtt_display_modes.get(&chan_num).unwrap_or(&RttDisplayMode::Text);
            
            egui::ScrollArea::vertical()
                .id_source("rtt_scroll")
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    match mode {
                        RttDisplayMode::Text => {
                            let buffer = self.rtt_buffers.entry(chan_num).or_insert_with(String::new);
                            ui.add(egui::TextEdit::multiline(buffer)
                                .font(egui::TextStyle::Monospace)
                                .code_editor()
                                .lock_focus(false)
                                .desired_width(f32::INFINITY)
                                .desired_rows(20));
                        }
                        RttDisplayMode::Hex => {
                            let raw = self.rtt_raw_buffers.entry(chan_num).or_insert_with(Vec::new);
                            let mut hex_text = String::new();
                            for chunk in raw.chunks(16) {
                                for byte in chunk {
                                    hex_text.push_str(&format!("{:02X} ", byte));
                                }
                                hex_text.push('\n');
                            }
                            ui.add(egui::TextEdit::multiline(&mut hex_text)
                                .font(egui::TextStyle::Monospace)
                                .code_editor()
                                .lock_focus(false)
                                .desired_width(f32::INFINITY)
                                .desired_rows(20));
                        }
                        _ => { ui.label("Binary mode not implemented yet"); }
                    }
                });

            ui.horizontal(|ui| {
                let response = ui.text_edit_singleline(&mut self.rtt_input);
                if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.button("Send").clicked() {
                    if let Some(handle) = &self.session_handle {
                        let _ = handle.send(aether_core::DebugCommand::RttWrite {
                            channel: chan_num,
                            data: self.rtt_input.as_bytes().to_vec(),
                        });
                        self.rtt_input.clear();
                    }
                }
            });
        }
    }

    fn highlight_file(&self, file_path: &Path, content: &str) -> Vec<egui::text::LayoutJob> {
        let syntax = self.syntax_set.find_syntax_for_file(file_path).unwrap_or(None)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        
        // Use a dark theme that usually comes with syntect defaults
        let theme = &self.theme_set.themes.get("base16-ocean.dark")
            .or_else(|| self.theme_set.themes.get("base16-eighties.dark"))
            .or_else(|| self.theme_set.themes.get("base16-mocha.dark"))
            .or_else(|| self.theme_set.themes.values().next()) // Fallback to any theme
            .unwrap();

        let mut highlighter = HighlightLines::new(syntax, theme);
        
        content.lines().map(|line| {
            let mut job = egui::text::LayoutJob::default();
            // Syntect doesn't handle newlines in highlight_line, so we process the line content
            let ranges = highlighter.highlight_line(line, &self.syntax_set).unwrap_or_default();
            
            for (style, text) in ranges {
                let fg = style.foreground;
                let color = egui::Color32::from_rgb(fg.r, fg.g, fg.b);
                job.append(text, 0.0, egui::TextFormat {
                    color,
                    font_id: egui::FontId::monospace(14.0),
                    ..Default::default()
                });
            }
            // If the line is empty, job is empty, which is fine, but for layout we might want a height?
            // egui Label handles empty layout jobs gracefully usually, but maybe add a zero-width space if needed.
            // But better to let the Grid handle row height.
            if job.text.is_empty() {
                 job.append(" ", 0.0, egui::TextFormat {
                    font_id: egui::FontId::monospace(14.0),
                    ..Default::default()
                });
            }
            job
        }).collect()
    }

    pub(crate) fn draw_source_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Source Code");
        
        ui.horizontal(|ui| {
             if ui.button("📂 Load Symbols (ELF)").clicked() {
                  if let Some(path) = rfd::FileDialog::new()
                      .add_filter("ELF", &["elf", "bin", "out"])
                      .pick_file() 
                  {
                      if let Some(handle) = &self.session_handle {
                          let _ = handle.send(aether_core::DebugCommand::LoadSymbols(path));
                      }
                  }
             }
             if self.symbols_loaded {
                 ui.label("✅ Symbols Loaded");
             }
        });

        ui.separator();

        if let Some(info) = &self.source_info {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("File: {}", info.file.display())).strong());
                ui.separator();
                ui.label(egui::RichText::new(format!("Function: {}", info.function.as_deref().unwrap_or("unknown"))).italics());
            });
            
            ui.separator();
            
            if let Some((_, highlighted)) = self.source_cache.get(&info.file) {
                egui::ScrollArea::vertical()
                    .id_source("source_scroll")
                    .show(ui, |ui| {
                        egui::Grid::new("source_grid")
                            .num_columns(2)
                            .striped(true)
                            .show(ui, |ui| {
                                for (i, job) in highlighted.iter().enumerate() {
                                    let line_num = i + 1;
                                    let is_current_line = line_num as u32 == info.line;
                                    
                                    // Check if line has a breakpoint
                                    let has_breakpoint = self.breakpoint_locations.iter().any(|bp| 
                                        bp.file == info.file && bp.line == line_num as u32
                                    );

                                    ui.horizontal(|ui| {
                                        ui.style_mut().visuals.override_text_color = Some(egui::Color32::GRAY);
                                        
                                        let mut label_text = egui::RichText::new(format!("{:4}", line_num));
                                        if has_breakpoint {
                                            label_text = label_text.color(egui::Color32::RED).strong();
                                        }

                                        if ui.add(egui::Label::new(label_text).sense(egui::Sense::click())).clicked() {
                                            if let Some(handle) = &self.session_handle {
                                                let _ = handle.send(aether_core::DebugCommand::ToggleBreakpointAtSource(info.file.clone(), line_num as u32));
                                            }
                                        }
                                        
                                        if has_breakpoint {
                                            ui.colored_label(egui::Color32::RED, "●");
                                        }
                                    });
                                    
                                    let mut line_job = job.clone();
                                    if is_current_line {
                                        let bg = egui::Color32::from_rgba_premultiplied(255, 255, 0, 50);
                                        for section in &mut line_job.sections {
                                            section.format.background = bg;
                                        }
                                    }
                                    
                                    ui.add(egui::Label::new(line_job));
                                    ui.end_row();
                                }
                            });
                    });
            } else {
                ui.label("Source file not found or failed to load.");
            }
        } else {
            ui.label("No source information available. Halt the target to see source code.");
        }
    }

    fn apply_midnight_theme(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        
        // Deep midnight colors
        visuals.panel_fill = egui::Color32::from_rgb(10, 12, 18);
        visuals.window_fill = egui::Color32::from_rgb(15, 18, 26);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(25, 30, 45);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(35, 45, 65);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(45, 60, 90);
        
        // Neon accents
        visuals.selection.bg_fill = egui::Color32::from_rgb(0, 150, 255);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 255));
        
        ctx.set_visuals(visuals);

        let mut style: egui::Style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.window_margin = egui::Margin::same(12.0);
        style.visuals.window_rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
        ctx.set_style(style);
    }
    pub(crate) fn draw_control_view(&mut self, ui: &mut egui::Ui) {
        // Connection Section
        ui.add_space(8.0);
        ui.collapsing("🔌 Connection", |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("🔄 Refresh").clicked() {
                        self.refresh_probes();
                    }
                    if ui.button("🔌 Connect").clicked() {
                        self.connect_probe();
                    }
                });
                
                egui::ScrollArea::vertical().id_source("probes").max_height(100.0).show(ui, |ui| {
                    for (i, probe) in self.probes.iter().enumerate() {
                        let is_selected = self.selected_probe == Some(i);
                        if ui.selectable_label(is_selected, format!("▷ {}", probe.name())).clicked() {
                            self.selected_probe = Some(i);
                        }
                    }
                });

                ui.separator();
                ui.label(egui::RichText::new("🌐 Remote").strong());
                ui.horizontal(|ui| {
                    ui.label("Host:");
                    ui.text_edit_singleline(&mut self.remote_host);
                });
                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.remote_port);
                });
                if ui.button("🚀 Connect Remote").clicked() {
                    self.connect_remote();
                }
            });
        });

        ui.add_space(8.0);
        
        // Core Control Section
        ui.group(|ui| {
            ui.heading("🕹 Core Control");
            ui.horizontal_wrapped(|ui| {
                let btn_size = egui::vec2(70.0, 30.0);
                
                ui.add_enabled_ui(self.session_handle.is_some(), |ui| {
                        if ui.add(egui::Button::new("⏸ Halt").min_size(btn_size)).clicked() {
                            let _ = self.session_handle.as_ref().unwrap().send(aether_core::DebugCommand::Halt);
                        }
                        if ui.add(egui::Button::new("▶ Resume").min_size(btn_size)).clicked() {
                            let _ = self.session_handle.as_ref().unwrap().send(aether_core::DebugCommand::Resume);
                        }
                        if ui.add(egui::Button::new("⏭ Step").min_size(btn_size)).clicked() {
                            let _ = self.session_handle.as_ref().unwrap().send(aether_core::DebugCommand::Step);
                        }
                        if ui.add(egui::Button::new("↷ Over").min_size(btn_size)).clicked() {
                            let _ = self.session_handle.as_ref().unwrap().send(aether_core::DebugCommand::StepOver);
                        }
                        if ui.add(egui::Button::new("↘ Into").min_size(btn_size)).clicked() {
                            let _ = self.session_handle.as_ref().unwrap().send(aether_core::DebugCommand::StepInto);
                        }
                        if ui.add(egui::Button::new("↗ Out").min_size(btn_size)).clicked() {
                            let _ = self.session_handle.as_ref().unwrap().send(aether_core::DebugCommand::StepOut);
                        }
                });
            });
        });

        ui.add_space(8.0);
        
        // Registers Section
        ui.collapsing("🔢 Registers", |ui| {
            egui::ScrollArea::vertical().id_source("regs").show(ui, |ui| {
                egui::Grid::new("reg_grid").striped(true).spacing(egui::vec2(20.0, 4.0)).show(ui, |ui| {
                    for i in 0..16 {
                        ui.label(egui::RichText::new(format!("R{}", i)).color(egui::Color32::from_rgb(0, 200, 255)));
                        if let Some(val) = self.registers.get(&i) {
                            ui.label(egui::RichText::new(format!("0x{:08X}", val)).monospace());
                        } else {
                            ui.label("?");
                        }
                        if i % 2 == 1 { ui.end_row(); }
                    }
                });
            });
        });

        ui.add_space(8.0);

        // Breakpoints Section
        ui.collapsing("🛑 Breakpoints", |ui| {
            self.draw_breakpoints_view(ui);
        });
        
        ui.add_space(8.0);
        
        // Flashing Section
        ui.collapsing("🚀 Flash Programming", |ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 File").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Binaries", &["bin", "elf", "hex"])
                        .pick_file()
                    {
                        self.selected_file = Some(path);
                    }
                }
                if let Some(file) = &self.selected_file {
                    ui.label(file.file_name().unwrap_or_default().to_string_lossy());
                }
            });

            if ui.add_enabled(self.selected_file.is_some() && self.connection_status == ConnectionStatus::Connected, egui::Button::new("🚀 Flash")).clicked() {
                self.start_flashing();
            }

            if let Some(p) = self.flashing_progress {
                ui.add(egui::ProgressBar::new(p).text(&self.flashing_status));
            }
        });
    }

    pub(crate) fn draw_logs_view(&mut self, ui: &mut egui::Ui) {
         ui.heading("System Logs");
         egui::ScrollArea::vertical().id_source("logs_scroll").show(ui, |ui| {
             ui.monospace(&self.status_message);
             for req in &self.failed_requests {
                  ui.label(egui::RichText::new(format!("Error: {}", req)).color(egui::Color32::RED));
             }
         });
         
         ui.add_space(8.0);
         ui.heading("Quick Connect (Python)");
         let code = r#"import grpc
import aether_pb2
import aether_pb2_grpc

channel = grpc.insecure_channel('localhost:50051')
stub = aether_pb2_grpc.AetherDebugStub(channel)

# Halt and Inspect (Synchronous)
stub.Halt(aether_pb2.Empty())
print(stub.ReadRegister(aether_pb2.ReadRegisterRequest(register_number=15)))
stub.Resume(aether_pb2.Empty())
"#;
         let mut code_buf = code.to_string();
         ui.add(egui::TextEdit::multiline(&mut code_buf).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
    }
}

impl eframe::App for AetherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_midnight_theme(ctx);
        self.update_flashing();
        self.process_debug_events();

        // Top Header
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("ÆTHER").strong().color(egui::Color32::from_rgb(0, 255, 255)));
                ui.label(egui::RichText::new("v0.1.0").small().color(egui::Color32::GRAY));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let status_color = match self.connection_status {
                        ConnectionStatus::Disconnected => egui::Color32::GRAY,
                        ConnectionStatus::Connecting => egui::Color32::YELLOW,
                        ConnectionStatus::Connected => egui::Color32::GREEN,
                        ConnectionStatus::Error => egui::Color32::RED,
                    };
                    
                    let status_dot = if self.connection_status == ConnectionStatus::Connecting { "◌" } else { "●" };
                    ui.label(egui::RichText::new(status_dot).color(status_color).strong());
                    ui.label(match self.connection_status {
                        ConnectionStatus::Disconnected => "Disconnected",
                        ConnectionStatus::Connecting => "Connecting...",
                        ConnectionStatus::Connected => "Connected",
                        ConnectionStatus::Error => "Connection Error",
                    });

                    if let Some(target) = &self.target_info {
                        ui.separator();
                        ui.label(egui::RichText::new(&target.name).strong());
                        ui.label("Target:");
                    }
                });
            });
            ui.add_space(4.0);
        });

        // Bottom Panel: Status
        egui::TopBottomPanel::bottom("bottom_status").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Log:").color(egui::Color32::GRAY));
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(status) = self.core_status {
                        ui.label(format!("State: {:?}", status));
                    }
                });

            });
            ui.add_space(4.0);
        });

        // Main Dock Area
        egui::CentralPanel::default().show(ctx, |ui| {
             if let Some(mut dock_state) = self.dock_state.take() {
                 let mut tab_viewer = AetherTabViewer { app: self };
                 DockArea::new(&mut dock_state)
                     .style(Style::from_egui(ctx.style().as_ref()))
                     .show_inside(ui, &mut tab_viewer);
                 self.dock_state = Some(dock_state);
             }
        });

        if self.progress_receiver.is_some() || self.session_handle.is_some() {
            ctx.request_repaint();
        }
    }
}
