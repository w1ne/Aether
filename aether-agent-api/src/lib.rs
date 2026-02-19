//! Aether Agent API crate.
//!
//! Provides the gRPC service and client for interacting with the Aether debugger core.

use aether_core::{DebugCommand, DebugEvent as CoreDebugEvent, SessionHandle};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tonic::{transport::Server, Request, Response, Status};

/// Generated gRPC code from protobuf definitions.
#[allow(missing_docs)]
#[allow(clippy::all)]
#[allow(clippy::pedantic)]
#[allow(clippy::nursery)]
pub mod proto {
    tonic::include_proto!("aether");
}

use proto::aether_debug_server::{AetherDebug, AetherDebugServer};
use proto::{
    AttachRequest, BreakpointList, BreakpointRequest, DebugEvent, DisasmRequest, DisasmResponse,
    Empty, FileRequest, FlashProgress, ItmConfig, ItmEvent, PeripheralRequest, PeripheralResponse,
    PeripheralWriteRequest, ProbeInfo as ProtoProbeInfo, ProbeList, ReadMemoryRequest,
    ReadMemoryResponse, ReadRegisterRequest, ReadRegisterResponse, RttWriteRequest,
    SemihostingEvent, StackResponse, StatusResponse, TasksEvent, WatchVariableRequest,
    WriteMemoryRequest, WriteRegisterRequest,
};

/// Service implementation for the Aether Debug gRPC API.
pub struct AetherDebugService {
    session: Arc<SessionHandle>,
}

impl AetherDebugService {
    /// Create a new `AetherDebugService` with a session handle.
    #[must_use]
    pub const fn new(session: Arc<SessionHandle>) -> Self {
        Self { session }
    }

    async fn wait_for_match<F>(
        &self,
        rx: &mut broadcast::Receiver<CoreDebugEvent>,
        matcher: F,
    ) -> Result<CoreDebugEvent, Status>
    where
        F: Fn(&CoreDebugEvent) -> bool + Send + 'static,
    {
        let timeout = Duration::from_secs(15); // Increased further to allow for multi-stage SWD/JTAG/Reset scan

        loop {
            match tokio::time::timeout(timeout, rx.recv()).await {
                Ok(Ok(event)) => {
                    if matcher(&event) {
                        return Ok(event);
                    }
                    if let CoreDebugEvent::Error(e) = event {
                        return Err(Status::internal(format!("Core error: {e}")));
                    }
                }
                Ok(Err(_)) => return Err(Status::internal("Event stream lagged or closed")),
                Err(_) => return Err(Status::deadline_exceeded("Timeout waiting for debug event")),
            }
        }
    }
}

#[tonic::async_trait]
impl AetherDebug for AetherDebugService {
    type SubscribeEventsStream = std::pin::Pin<
        Box<dyn tokio_stream::Stream<Item = Result<DebugEvent, Status>> + Send + Sync>,
    >;

    async fn halt(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::Halt).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn resume(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::Resume).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn step(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::Step).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn step_over(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::StepOver).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn step_into(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::StepInto).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn step_out(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::StepOut).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn reset(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::Reset).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn get_status(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<StatusResponse>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::PollStatus).map_err(|e| Status::internal(e.to_string()))?;

        let mut halted = false;
        let mut pc = 0;
        let mut core_status = "Unknown".to_string();
        let mut received_status = false;

        // Wait for status/halted events
        let _ = tokio::time::timeout(Duration::from_millis(500), async {
            while let Ok(event) = rx.recv().await {
                match event {
                    CoreDebugEvent::Status(s) => {
                        halted = s.is_halted();
                        core_status = format!("{s:?}");
                        received_status = true;
                        // If not halted, we are done (no PC). If halted, we might want to wait for PC if not received yet.
                        if !halted {
                            break;
                        }
                        if pc != 0 {
                            break;
                        }
                    }
                    CoreDebugEvent::Halted { pc: p } => {
                        pc = p;
                        halted = true;
                        if received_status {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        })
        .await;

        Ok(Response::new(StatusResponse { halted, pc, core_status }))
    }

    async fn read_memory(
        &self,
        request: Request<ReadMemoryRequest>,
    ) -> Result<Response<ReadMemoryResponse>, Status> {
        let req = request.into_inner();
        let mut rx = self.session.subscribe();
        self.session
            .send(DebugCommand::ReadMemory(req.address, req.length as usize))
            .map_err(|e| Status::internal(e.to_string()))?;

        let event =
            self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::MemoryData(..))).await?;
        if let CoreDebugEvent::MemoryData(_, data) = event {
            Ok(Response::new(ReadMemoryResponse { data }))
        } else {
            Err(Status::internal("Unexpected event"))
        }
    }

    async fn read_register(
        &self,
        request: Request<ReadRegisterRequest>,
    ) -> Result<Response<ReadRegisterResponse>, Status> {
        let req = request.into_inner();
        let mut rx = self.session.subscribe();
        self.session
            .send(DebugCommand::ReadRegister(u16::try_from(req.register_number).unwrap_or(0)))
            .map_err(|e| Status::internal(e.to_string()))?;

        let event = self
            .wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::RegisterValue(..)))
            .await?;
        if let CoreDebugEvent::RegisterValue(_, value) = event {
            Ok(Response::new(ReadRegisterResponse { value }))
        } else {
            Err(Status::internal("Unexpected event"))
        }
    }

    async fn write_memory(
        &self,
        _request: Request<WriteMemoryRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("WriteMemory not implemented"))
    }

    async fn write_register(
        &self,
        _request: Request<WriteRegisterRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("WriteRegister not implemented"))
    }

    async fn load_svd(&self, request: Request<FileRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let mut rx = self.session.subscribe();
        self.session
            .send(DebugCommand::LoadSvd(std::path::PathBuf::from(req.path)))
            .map_err(|e| Status::internal(e.to_string()))?;

        let _ = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::SvdLoaded)).await?;
        Ok(Response::new(Empty {}))
    }

    async fn get_peripherals(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<PeripheralResponse>, Status> {
        Err(Status::unimplemented("GetPeripherals not implemented"))
    }

    async fn read_peripheral(
        &self,
        _request: Request<PeripheralRequest>,
    ) -> Result<Response<proto::RegisterList>, Status> {
        Err(Status::unimplemented("ReadPeripheral not implemented"))
    }

    async fn write_peripheral(
        &self,
        _request: Request<PeripheralWriteRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("WritePeripheral not implemented"))
    }

    async fn set_breakpoint(
        &self,
        _request: Request<BreakpointRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("SetBreakpoint not implemented"))
    }

    async fn clear_breakpoint(
        &self,
        _request: Request<BreakpointRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("ClearBreakpoint not implemented"))
    }

    async fn list_breakpoints(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<BreakpointList>, Status> {
        Err(Status::unimplemented("ListBreakpoints not implemented"))
    }

    async fn watch_variable(
        &self,
        request: Request<WatchVariableRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session
            .send(DebugCommand::WatchVariable(req.name))
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn rtt_write(
        &self,
        _request: Request<RttWriteRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("RttWrite not implemented"))
    }

    async fn get_tasks(&self, _request: Request<Empty>) -> Result<Response<TasksEvent>, Status> {
        Err(Status::unimplemented("GetTasks not implemented"))
    }

    async fn get_stack(&self, _request: Request<Empty>) -> Result<Response<StackResponse>, Status> {
        let mut rx = self.session.subscribe();
        self.session
            .send(DebugCommand::GetStack)
            .map_err(|e| Status::internal(e.to_string()))?;

        let event =
            self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Stack(_))).await?;

        if let CoreDebugEvent::Stack(frames) = event {
            let proto_frames = frames
                .into_iter()
                .map(|f| proto::StackFrame {
                    pc: f.pc as u64,
                    function_name: Some(f.function_name),
                    file: f.source_file,
                    line: f.line.map(|l| l as u32),
                })
                .collect();
            Ok(Response::new(StackResponse { frames: proto_frames }))
        } else {
            Err(Status::internal("Unexpected event"))
        }
    }

    async fn load_symbols(&self, request: Request<FileRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session
            .send(DebugCommand::LoadSymbols(std::path::PathBuf::from(req.path)))
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    type FlashStream = std::pin::Pin<
        Box<dyn tokio_stream::Stream<Item = Result<FlashProgress, Status>> + Send + 'static>,
    >;

    async fn flash(
        &self,
        request: Request<FileRequest>,
    ) -> Result<Response<Self::FlashStream>, Status> {
        let req = request.into_inner();
        let path = std::path::PathBuf::from(req.path);

        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let mut session_rx = self.session.subscribe();

        // Send flash command to core
        if let Err(e) = self.session.send(DebugCommand::StartFlashing(path)) {
            return Err(Status::internal(e.to_string()));
        }

        tokio::spawn(async move {
            while let Ok(event) = session_rx.recv().await {
                let progress = match event {
                    aether_core::DebugEvent::FlashStatus(s) => FlashProgress {
                        status: s,
                        progress: 0.0,
                        done: false,
                        error: String::new(),
                    },
                    aether_core::DebugEvent::FlashProgress(p) => FlashProgress {
                        status: "Flashing".to_string(),
                        progress: p,
                        done: false,
                        error: String::new(),
                    },
                    aether_core::DebugEvent::FlashDone => {
                        let _ = tx
                            .send(Ok(FlashProgress {
                                status: "Done".to_string(),
                                progress: 1.0,
                                done: true,
                                error: String::new(),
                            }))
                            .await;
                        break;
                    }
                    aether_core::DebugEvent::Error(e) => {
                        let _ = tx
                            .send(Ok(FlashProgress {
                                status: "Error".to_string(),
                                progress: 0.0,
                                done: true,
                                error: e,
                            }))
                            .await;
                        break;
                    }
                    _ => continue,
                };
                if tx.send(Ok(progress)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))))
    }

    async fn disassemble(
        &self,
        request: Request<DisasmRequest>,
    ) -> Result<Response<DisasmResponse>, Status> {
        let req = request.into_inner();
        // Disassembly is tricky because it returns via event usually.
        // We'll implemented a request-response pattern by waiting for the specific event.
        // This acts as a bridge.

        let mut session_rx = self.session.subscribe();
        self.session
            .send(DebugCommand::Disassemble(req.address, req.count as usize))
            .map_err(|e| Status::internal(e.to_string()))?;

        // Wait for response with timeout
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while let Ok(event) = session_rx.recv().await {
                if let aether_core::DebugEvent::Disassembly(lines) = event {
                    let instructions = lines
                        .iter()
                        .map(|l| format!("0x{:08X}:  {}  {}", l.address, l.mnemonic, l.op_str))
                        .collect();
                    return Ok(DisasmResponse { instructions });
                }
            }
            Err(Status::internal("Stream closed"))
        })
        .await;

        match result {
            Ok(Ok(resp)) => Ok(Response::new(resp)),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(Status::deadline_exceeded("Disassembly timed out")),
        }
    }

    async fn enable_semihosting(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Empty>, Status> {
        self.session
            .send(DebugCommand::EnableSemihosting)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn enable_itm(&self, request: Request<ItmConfig>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session
            .send(DebugCommand::EnableItm { baud_rate: req.baud_rate })
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn list_probes(&self, _request: Request<Empty>) -> Result<Response<ProbeList>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::ListProbes).map_err(|e| Status::internal(e.to_string()))?;

        let event =
            self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Probes(_))).await?;

        if let CoreDebugEvent::Probes(probes) = event {
            let proto_probes = probes
                .into_iter()
                .enumerate()
                .map(|(i, p)| ProtoProbeInfo {
                    index: u32::try_from(i).unwrap_or(0),
                    name: p.name(),
                    serial: p.serial_number.unwrap_or_default(),
                })
                .collect();
            Ok(Response::new(ProbeList { probes: proto_probes }))
        } else {
            Err(Status::internal("Unexpected event"))
        }
    }

    async fn attach(&self, request: Request<AttachRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let mut rx = self.session.subscribe();

        let protocol = match req.protocol.as_deref() {
            Some("swd") => Some(aether_core::WireProtocol::Swd),
            Some("jtag") => Some(aether_core::WireProtocol::Jtag),
            _ => None,
        };

        self.session
            .send(DebugCommand::Attach {
                probe_index: req.probe_index as usize,
                chip: req.chip,
                protocol,
                under_reset: req.under_reset,
            })
            .map_err(|e| Status::internal(e.to_string()))?;

        let _ = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Attached(_))).await?;
        Ok(Response::new(Empty {}))
    }

    // --- Events ---

    async fn subscribe_events(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        let rx = self.session.subscribe();
        let stream = BroadcastStream::new(rx);

        let output = stream.filter_map(|res| {
            res.ok().and_then(|core_event| map_core_event_to_proto(core_event).map(Ok))
        });

        Ok(Response::new(Box::pin(output)))
    }
}

/// Maps a core debug event to a protocol buffer debug event.
#[must_use]
pub fn map_core_event_to_proto(event: CoreDebugEvent) -> Option<DebugEvent> {
    match event {
        CoreDebugEvent::Halted { pc } => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Halted(proto::HaltedEvent { pc })),
        }),
        CoreDebugEvent::Resumed => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Resumed(proto::ResumedEvent {})),
        }),
        CoreDebugEvent::MemoryData(address, data) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Memory(proto::MemoryEvent { address, data })),
        }),
        CoreDebugEvent::RegisterValue(address, value) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Register(proto::RegisterEvent {
                register: u32::from(address),
                value,
            })),
        }),
        CoreDebugEvent::Tasks(tasks) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Tasks(proto::TasksEvent {
                tasks: tasks
                    .into_iter()
                    .map(|t| proto::TaskInfo {
                        name: t.name,
                        priority: t.priority,
                        state: format!("{:?}", t.state),
                        stack_usage: t.stack_usage,
                        stack_size: t.stack_size,
                        handle: t.handle,
                        task_type: format!("{:?}", t.task_type),
                    })
                    .collect(),
            })),
        }),
        CoreDebugEvent::TaskSwitch { from, to, timestamp } => Some(DebugEvent {
            event: Some(proto::debug_event::Event::TaskSwitch(proto::TaskSwitchEvent {
                from,
                to,
                timestamp,
            })),
        }),
        CoreDebugEvent::PlotData { name, timestamp, value } => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Plot(proto::PlotEvent {
                name,
                timestamp,
                value,
            })),
        }),
        CoreDebugEvent::RttData(channel, data) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Rtt(proto::RttEvent {
                channel: u32::try_from(channel).unwrap_or(0),
                data,
            })),
        }),
        CoreDebugEvent::SemihostingOutput(output) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Semihosting(SemihostingEvent { output })),
        }),
        CoreDebugEvent::ItmPacket(data) => {
            Some(DebugEvent { event: Some(proto::debug_event::Event::Itm(ItmEvent { data })) })
        }
        CoreDebugEvent::Probes(probes) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Probes(proto::ProbeList {
                probes: probes
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| proto::ProbeInfo {
                        index: u32::try_from(i).unwrap_or(0),
                        name: p.name(),
                        serial: p.serial_number.unwrap_or_default(),
                    })
                    .collect(),
            })),
        }),
        CoreDebugEvent::Attached(info) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Attached(proto::TargetInfo {
                name: info.name,
                flash_size: info.flash_size,
                ram_size: info.ram_size,
                architecture: info.architecture,
            })),
        }),
        CoreDebugEvent::VariableResolved(info) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Variable(map_type_info_to_proto(&info))),
        }),
        CoreDebugEvent::Status(s) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Status(proto::StatusResponse {
                halted: s.is_halted(),
                pc: 0,
                core_status: format!("{s:?}"),
            })),
        }),
        _ => None,
    }
}

/// Helper to map `aether_core::symbols::TypeInfo` into `proto::VariableEvent`
fn map_type_info_to_proto(info: &aether_core::symbols::TypeInfo) -> proto::VariableEvent {
    proto::VariableEvent {
        name: info.name.clone(),
        value: info.value_formatted_string.clone(),
        r#type: info.kind.clone(),
        members: info
            .members
            .as_ref()
            .map(|m| m.iter().map(map_type_info_to_proto).collect())
            .unwrap_or_default(),
        address: info.address,
    }
}

/// Maps a protocol buffer debug event back to a core debug event.
#[must_use]
pub fn map_proto_event_to_core(event: DebugEvent) -> Option<CoreDebugEvent> {
    match event.event? {
        proto::debug_event::Event::Halted(h) => Some(CoreDebugEvent::Halted { pc: h.pc }),
        proto::debug_event::Event::Resumed(_) => Some(CoreDebugEvent::Resumed),
        proto::debug_event::Event::Memory(m) => Some(CoreDebugEvent::MemoryData(m.address, m.data)),
        proto::debug_event::Event::Register(r) => {
            Some(CoreDebugEvent::RegisterValue(u16::try_from(r.register).unwrap_or(0), r.value))
        }
        proto::debug_event::Event::Tasks(t) => Some(CoreDebugEvent::Tasks(
            t.tasks
                .into_iter()
                .map(|ti| aether_core::TaskInfo {
                    name: ti.name,
                    priority: ti.priority,
                    state: match ti.state.as_str() {
                        "Running" => aether_core::TaskState::Running,
                        "Blocked" => aether_core::TaskState::Blocked,
                        _ => aether_core::TaskState::Ready,
                    },
                    stack_usage: ti.stack_usage,
                    stack_size: ti.stack_size,
                    handle: ti.handle,
                    task_type: if ti.task_type == "Async" {
                        aether_core::TaskType::Async
                    } else {
                        aether_core::TaskType::Thread
                    },
                })
                .collect(),
        )),
        proto::debug_event::Event::TaskSwitch(ts) => {
            Some(CoreDebugEvent::TaskSwitch { from: ts.from, to: ts.to, timestamp: ts.timestamp })
        }
        proto::debug_event::Event::Plot(p) => {
            Some(CoreDebugEvent::PlotData { name: p.name, timestamp: p.timestamp, value: p.value })
        }
        proto::debug_event::Event::Rtt(r) => {
            Some(CoreDebugEvent::RttData(r.channel as usize, r.data))
        }
        proto::debug_event::Event::Breakpoint(_)
        | proto::debug_event::Event::Variable(_)
        | proto::debug_event::Event::Status(_) => None,
        proto::debug_event::Event::Semihosting(s) => {
            Some(CoreDebugEvent::SemihostingOutput(s.output))
        }
        proto::debug_event::Event::Itm(i) => Some(CoreDebugEvent::ItmPacket(i.data)),
        proto::debug_event::Event::Probes(p) => Some(CoreDebugEvent::Probes(
            p.probes
                .into_iter()
                .map(|pi| aether_core::ProbeInfo {
                    vendor_id: 0,
                    product_id: 0,
                    serial_number: if pi.serial.is_empty() { None } else { Some(pi.serial) },
                    probe_type: aether_core::ProbeType::Other,
                })
                .collect(),
        )),
        proto::debug_event::Event::Attached(i) => {
            Some(CoreDebugEvent::Attached(aether_core::TargetInfo {
                name: i.name,
                flash_size: i.flash_size,
                ram_size: i.ram_size,
                architecture: i.architecture,
            }))
        }
    }
}

/// Runs the gRPC server on the specified host and port.
pub async fn run_server(
    session: Arc<SessionHandle>,
    host: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{host}:{port}").parse()?;
    let service = AetherDebugService::new(session);

    println!("Agent API Server listening on {addr}");

    Server::builder().add_service(AetherDebugServer::new(service)).serve(addr).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_core::DebugEvent as CoreDebugEvent;

    #[test]
    fn test_event_mapping_halted() {
        let core_event = CoreDebugEvent::Halted { pc: 0x1234 };
        let proto_event = map_core_event_to_proto(core_event).unwrap();
        if let Some(proto::debug_event::Event::Halted(h)) = proto_event.event {
            assert_eq!(h.pc, 0x1234);
        } else {
            panic!("Wrong event type");
        }
    }

    // Adding this just as an example since GetStack maps directly without `map_core_event_to_proto`
    // but we can test the general struct initialization.
    #[test]
    fn test_stack_frame_mapping() {
        let core_frame = aether_core::StackFrame {
            id: 1,
            function_name: "main".to_string(),
            source_file: Some("src/main.rs".to_string()),
            line: Some(42),
            pc: 0x0800_1234,
            sp: 0x2000_1000,
        };

        let proto_frame = proto::StackFrame {
            pc: core_frame.pc as u64,
            function_name: Some(core_frame.function_name),
            file: core_frame.source_file,
            line: core_frame.line.map(|l| l as u32),
        };

        assert_eq!(proto_frame.pc, 0x0800_1234);
        assert_eq!(proto_frame.function_name.unwrap(), "main");
        assert_eq!(proto_frame.file.unwrap(), "src/main.rs");
        assert_eq!(proto_frame.line.unwrap(), 42);
    }

    #[test]
    fn test_event_mapping_resumed() {
        let core_event = CoreDebugEvent::Resumed;
        let proto_event = map_core_event_to_proto(core_event).unwrap();
        assert!(matches!(proto_event.event, Some(proto::debug_event::Event::Resumed(_))));
    }

    #[test]
    fn test_event_mapping_memory() {
        let core_event = CoreDebugEvent::MemoryData(0x2000, vec![1, 2, 3]);
        let proto_event = map_core_event_to_proto(core_event).unwrap();
        if let Some(proto::debug_event::Event::Memory(m)) = proto_event.event {
            assert_eq!(m.address, 0x2000);
            assert_eq!(m.data, vec![1, 2, 3]);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_variable_mapping() {
        let type_info = aether_core::symbols::TypeInfo {
            name: "my_var".to_string(),
            value_formatted_string: "0x1234".to_string(),
            kind: "Primitive".to_string(),
            members: None,
            address: Some(0x2000_0000),
        };

        let core_event = CoreDebugEvent::VariableResolved(type_info);
        let proto_event = map_core_event_to_proto(core_event).unwrap();

        if let Some(proto::debug_event::Event::Variable(v)) = proto_event.event {
            assert_eq!(v.name, "my_var");
            assert_eq!(v.value, "0x1234");
            assert_eq!(v.r#type, "Primitive");
            assert!(v.members.is_empty());
            assert_eq!(v.address.unwrap(), 0x2000_0000);
        } else {
            panic!("Wrong event type");
        }
    }
}
