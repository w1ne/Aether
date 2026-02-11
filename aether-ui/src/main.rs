use eframe::egui;

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
    connection_status: ConnectionStatus,
    status_message: String,
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
            connection_status: ConnectionStatus::Disconnected,
            status_message: String::new(),
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
                Ok(_probe) => {
                    self.connection_status = ConnectionStatus::Connected;
                    self.status_message = format!("Connected to {}", self.probes[index].name());
                }
                Err(e) => {
                    self.connection_status = ConnectionStatus::Error;
                    self.status_message = format!("Failed to connect: {}", e);
                }
            }
        }
    }
}

impl eframe::App for AetherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

            // Probe selection
            ui.heading("Debug Probes");

            ui.horizontal(|ui| {
                if ui.button("🔄 Refresh").clicked() {
                    self.refresh_probes();
                }

                if ui.button("🔌 Connect").clicked() {
                    self.connect_probe();
                }
            });

            ui.separator();

            // Probe list
            if self.probes.is_empty() {
                ui.label("No probes found. Click Refresh to scan.");
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (index, probe) in self.probes.iter().enumerate() {
                        let is_selected = self.selected_probe == Some(index);
                        if ui.selectable_label(is_selected, &probe.name()).clicked() {
                            self.selected_probe = Some(index);
                        }

                        ui.indent(format!("probe_{}", index), |ui| {
                            ui.label(format!(
                                "VID:PID = {:04X}:{:04X}",
                                probe.vendor_id, probe.product_id
                            ));
                            if let Some(ref serial) = probe.serial_number {
                                ui.label(format!("Serial: {}", serial));
                            }
                        });
                        ui.separator();
                    }
                });
            }
        });
    }
}
