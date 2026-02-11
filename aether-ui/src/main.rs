use crossbeam_channel::{unbounded, Receiver};
use eframe::egui;
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

    // Flashing state
    selected_file: Option<PathBuf>,
    flashing_progress: Option<f32>,
    flashing_status: String,
    progress_receiver: Option<Receiver<aether_core::FlashingProgress>>,
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
            selected_file: None,
            flashing_progress: None,
            flashing_status: String::new(),
            progress_receiver: None,
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
                    match self.probe_manager.detect_target(probe) {
                        Ok(target) => {
                            self.target_info = Some(target.clone());
                            self.connection_status = ConnectionStatus::Connected;
                            self.status_message = format!(
                                "Connected to {} -> {}",
                                self.probes[index].name(),
                                target.name
                            );
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

        let (tx, rx) = unbounded();
        self.progress_receiver = Some(rx);
        self.flashing_progress = Some(0.0);
        self.flashing_status = "Preparing to flash...".to_string();

        // In a real app, we'd take the session from probe_manager or similar
        // For now, let's reopen the probe in the background thread
        let probe_index = self.selected_probe.unwrap();
        let flash_manager = aether_core::FlashManager::new();

        std::thread::spawn(move || {
            let probe_manager = aether_core::ProbeManager::new();
            match probe_manager.open_probe(probe_index) {
                Ok(probe) => {
                    match probe.attach("any", probe_rs::Permissions::default()) {
                        // "any" will use the auto-detected target from before
                        Ok(mut session) => {
                            let (mpsc_tx, mpsc_rx) = mpsc::channel();
                            let progress =
                                aether_core::MpscFlashProgress::new(mpsc_tx).into_flash_progress();

                            // Forward progress in a separate small thread
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
}

impl eframe::App for AetherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_flashing();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Aether Debugger v0.1.0");
            ui.separator();

            // Connection status
            ui.horizontal(|ui| {
                let status_color = match self.connection_status {
                    ConnectionStatus::Disconnected => egui::Color32::GRAY,
                    ConnectionStatus::Connecting => egui::Color32::YELLOW,
                    ConnectionStatus::Connected => egui::Color32::GREEN,
                    ConnectionStatus::Error => egui::Color32::RED,
                };
                ui.colored_label(status_color, "●");
                ui.label(&self.status_message);
            });

            ui.separator();

            // Sidebar-like layout
            ui.columns(2, |columns| {
                // Left Column: Probe Info
                columns[0].vertical(|ui| {
                    ui.heading("Debug Probes");
                    ui.horizontal(|ui| {
                        if ui.button("🔄 Refresh").clicked() {
                            self.refresh_probes();
                        }
                        if ui.button("🔌 Connect").clicked() {
                            self.connect_probe();
                        }
                    });

                    if self.probes.is_empty() {
                        ui.label("No probes found.");
                    } else {
                        egui::ScrollArea::vertical().id_source("probe_list").show(ui, |ui| {
                            for (index, probe) in self.probes.iter().enumerate() {
                                let is_selected = self.selected_probe == Some(index);
                                if ui.selectable_label(is_selected, probe.name()).clicked() {
                                    self.selected_probe = Some(index);
                                }
                            }
                        });
                    }

                    if let Some(ref target) = self.target_info {
                        ui.separator();
                        ui.heading("Target info");
                        ui.label(format!("Chip: {}", target.name));
                        ui.label(format!("Arch: {}", target.architecture));
                    }
                });

                // Right Column: Flash Operations
                columns[1].vertical(|ui| {
                    ui.heading("Flash Programming");

                    ui.horizontal(|ui| {
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
                        |ui| {
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
                });
            });
        });

        // Request a repaint if we're flashing to keep the progress bar moving
        if self.progress_receiver.is_some() {
            ctx.request_repaint();
        }
    }
}
