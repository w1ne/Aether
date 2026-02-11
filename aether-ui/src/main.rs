use crossbeam_channel::{unbounded, Receiver};
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;

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

struct AetherApp {
    probe_manager: aether_core::ProbeManager,
    probes: Vec<aether_core::ProbeInfo>,
    selected_probe: Option<usize>,
    target_info: Option<aether_core::TargetInfo>,
    connection_status: ConnectionStatus,
    status_message: String,

    // Session & Debug state
    session_handle: Option<aether_core::SessionHandle>,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl AetherApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            probe_manager: aether_core::ProbeManager::new(),
            probes: Vec::new(),
            selected_probe: None,
            target_info: None,
            connection_status: ConnectionStatus::Disconnected,
            status_message: "Ready".to_string(),
            session_handle: None,
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

    fn connect_probe(&mut self) {
        if let Some(index) = self.selected_probe {
            self.connection_status = ConnectionStatus::Connecting;
            match self.probe_manager.open_probe(index) {
                Ok(probe) => {
                    self.status_message =
                        format!("Connected to {}. Detecting target...", self.probes[index].name());
                    
                    // Detect target first - consumes probe, returns (info, session)
                    match self.probe_manager.detect_target(probe) {
                        Ok((target, session)) => {
                            self.target_info = Some(target.clone());
                            self.status_message = format!(
                                "Connected to {} -> {}",
                                self.probes[index].name(),
                                target.name
                            );

                            // Create SessionHandle which consumes the session
                            match aether_core::SessionHandle::new(session) {
                                Ok(handle) => {
                                     self.session_handle = Some(handle);
                                     self.connection_status = ConnectionStatus::Connected;
                                     // Initial Poll
                                     if let Some(h) = &self.session_handle {
                                         let _ = h.send(aether_core::DebugCommand::PollStatus);
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
        if let Some(handle) = &self.session_handle {
            while let Some(event) = handle.try_recv() {
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
                    }
                    aether_core::DebugEvent::Resumed => {
                        self.status_message = "Running...".to_string();
                        // Update status
                       let _ = handle.send(aether_core::DebugCommand::PollStatus);
                    }
                    aether_core::DebugEvent::RegisterValue { address, value } => {
                        self.registers.insert(address, value);
                    }
                    aether_core::DebugEvent::MemoryContent { address, data } => {
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
                        self.peripherals = periphs;
                    }
                    aether_core::DebugEvent::Registers(regs) => {
                        self.peripheral_registers = regs;
                    }
                    aether_core::DebugEvent::Error(e) => {
                         self.failed_requests.push(e.clone());
                         log::error!("Debug Error: {}", e);
                    }
                }
            }
        }
    }

    fn draw_memory_view(&mut self, ui: &mut egui::Ui) {
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
                 
                 let hex_part: String = chunk.iter()
                     .map(|b| format!("{:02X} ", b))
                     .collect();
                     
                 let ascii_part: String = chunk.iter()
                     .map(|b| if *b >= 32 && *b <= 126 { *b as char } else { '.' })
                     .collect();
                 
                 let padded_hex = format!("{:48}", hex_part); 
                 
                 ui.monospace(format!("{:08X}   {} {}", addr, padded_hex, ascii_part));
             }
        });
    }

    fn draw_disassembly_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Disassembly");
        
        egui::ScrollArea::vertical().id_source("disasm").show(ui, |ui| {
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

    fn draw_breakpoints_view(&mut self, ui: &mut egui::Ui) {
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

    fn draw_peripherals_view(&mut self, ui: &mut egui::Ui) {
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
                                      let mut field_text = format!("{}: [{}..{}]", 
                                          field.name, 
                                          field.bit_offset, 
                                          field.bit_offset + field.bit_width - 1);
                                      
                                  if let Some(val) = reg.value {
                                       let field_val = field.decode(val);
                                       field_text = format!("{} = 0x{:X}", field_text, field_val);
                                  }
                                      
                                      ui.label(field_text);
                                 }
                            });
                       }
                  }
             });
        } else {
             ui.label("Select a peripheral to view registers");
        }
    }
}

impl eframe::App for AetherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_flashing();
        self.process_debug_events();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Aether Debugger v0.1.0");
            ui.separator();

            // Toolbar
            ui.horizontal(|ui| {
                 let status_color = match self.connection_status {
                    ConnectionStatus::Disconnected => egui::Color32::GRAY,
                    ConnectionStatus::Connecting => egui::Color32::YELLOW,
                    ConnectionStatus::Connected => egui::Color32::GREEN,
                    ConnectionStatus::Error => egui::Color32::RED,
                };
                ui.colored_label(status_color, "●");
                ui.label(&self.status_message);
                
                ui.separator();
                
                if let Some(handle) = &self.session_handle {
                     if ui.button("⏸ Halt").clicked() {
                         let _ = handle.send(aether_core::DebugCommand::Halt);
                     }
                     if ui.button("▶ Resume").clicked() {
                         let _ = handle.send(aether_core::DebugCommand::Resume);
                     }
                     if ui.button("⏭ Step").clicked() {
                         let _ = handle.send(aether_core::DebugCommand::Step);
                     }
                } else {
                    ui.add_enabled(false, egui::Button::new("⏸ Halt"));
                    ui.add_enabled(false, egui::Button::new("▶ Resume"));
                    ui.add_enabled(false, egui::Button::new("⏭ Step"));
                }
            });

            ui.separator();

            egui::ScrollArea::both().show(ui, |ui: &mut egui::Ui| {
                ui.columns(5, |columns: &mut [egui::Ui]| {

                // Col 0: Connection & Probes
                columns[0].vertical(|ui: &mut egui::Ui| {
                    ui.heading("Connection");
                    ui.horizontal(|ui: &mut egui::Ui| {
                        if ui.button("🔄 Refresh").clicked() {
                            self.refresh_probes();
                        }
                        if ui.button("🔌 Connect").clicked() {
                            self.connect_probe();
                        }
                    });

                    ui.separator();

                    egui::ScrollArea::vertical().id_source("probes").show(ui, |ui: &mut egui::Ui| {
                        for (i, probe) in self.probes.iter().enumerate() {
                            let is_selected = self.selected_probe == Some(i);
                            if ui.selectable_label(is_selected, probe.name()).clicked() {
                                self.selected_probe = Some(i);
                            }
                        }
                    });

                    ui.separator();
                    ui.label(format!("Status: {:?}", self.connection_status));
                    ui.label(&self.status_message);

                    if let Some(target) = &self.target_info {
                        ui.separator();
                        ui.heading("Target Info");
                        ui.label(format!("Name: {}", target.name));
                        ui.label(format!("Arch: {}", target.architecture));
                        ui.label(format!("Flash: {} KB", target.flash_size / 1024));
                        ui.label(format!("RAM: {} KB", target.ram_size / 1024));
                    }
                });

                // Col 1: Core Control & Registers
                columns[1].vertical(|ui: &mut egui::Ui| {
                    ui.heading("Core Control");
                    ui.horizontal(|ui: &mut egui::Ui| {
                        if ui.button("⏸ Halt").clicked() {
                             if let Some(handle) = &self.session_handle {
                                 let _ = handle.send(aether_core::DebugCommand::Halt);
                             }
                        }
                        if ui.button("▶ Resume").clicked() {
                             if let Some(handle) = &self.session_handle {
                                 let _ = handle.send(aether_core::DebugCommand::Resume);
                             }
                        }
                        if ui.button("➡ Step").clicked() {
                             if let Some(handle) = &self.session_handle {
                                 let _ = handle.send(aether_core::DebugCommand::Step);
                             }
                        }
                    });

                    ui.separator();
                    ui.heading("Registers");
                    egui::ScrollArea::vertical().id_source("regs").show(ui, |ui: &mut egui::Ui| {
                        egui::Grid::new("reg_grid").striped(true).show(ui, |ui: &mut egui::Ui| {
                            ui.label("Reg");
                            ui.label("Value");
                            ui.end_row();
                            
                            for i in 0..16 {
                                ui.label(format!("R{}", i));
                                if let Some(val) = self.registers.get(&i) {
                                    ui.label(format!("0x{:08X}", val));
                                } else {
                                    ui.label("?");
                                }
                                ui.end_row();
                            }
                        });
                    });
                    
                    ui.separator();
                    self.draw_breakpoints_view(ui);
                });

                // Col 2: Memory View
                columns[2].vertical(|ui: &mut egui::Ui| {
                    self.draw_memory_view(ui);
                });

                // Col 3: Flash & Disassembly
                 columns[3].vertical(|ui: &mut egui::Ui| {
                    ui.heading("Flash Programming");

                    ui.horizontal(|ui: &mut egui::Ui| {
                        if ui.button("📂 Select File").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Binaries", &["bin", "elf", "hex"])
                                .pick_file()
                            {
                                self.selected_file = Some(path);
                            }
                        }

                        if let Some(file) = &self.selected_file {
                            ui.label(file.file_name().unwrap_or_default().to_string_lossy());
                        } else {
                            ui.label("No file selected");
                        }
                    });

                    ui.add_enabled_ui(
                        self.selected_file.is_some()
                            && self.connection_status == ConnectionStatus::Connected,
                        |ui: &mut egui::Ui| {
                            if ui.button("🚀 Flash Target").clicked() {
                                self.start_flashing();
                            }
                        },
                    );

                    if let Some(p) = self.flashing_progress {
                        ui.add(egui::ProgressBar::new(p).text(&self.flashing_status));
                    } else if !self.flashing_status.is_empty() {
                        ui.label(&self.flashing_status);
                    }

                    ui.separator();
                    self.draw_disassembly_view(ui);
                });

                // Col 4: Peripherals
                columns[4].vertical(|ui: &mut egui::Ui| {
                    self.draw_peripherals_view(ui);
                });
                });
            });
        });

        if self.progress_receiver.is_some() || self.session_handle.is_some() {
            ctx.request_repaint();
        }
    }
}
