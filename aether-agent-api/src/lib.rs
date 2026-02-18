use tonic::{transport::Server, Request, Response, Status};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use aether_core::{SessionHandle, DebugCommand, DebugEvent as CoreDebugEvent};
use std::sync::Arc;
use std::time::Duration;

pub mod proto {
    tonic::include_proto!("aether");
}

use proto::aether_debug_server::{AetherDebug, AetherDebugServer};
use proto::{
    Empty, StatusResponse, ReadMemoryRequest, ReadMemoryResponse, WriteMemoryRequest,
    ReadRegisterRequest, ReadRegisterResponse, WriteRegisterRequest,
    BreakpointRequest, BreakpointList, StackResponse,
    TasksEvent, PeripheralRequest, PeripheralResponse, PeripheralWriteRequest,
    WatchVariableRequest, RttWriteRequest, DebugEvent,
    FileRequest, FlashProgress, DisasmRequest, DisasmResponse,
    ItmConfig, SemihostingEvent, ItmEvent,
    ProbeList, ProbeInfo as ProtoProbeInfo, AttachRequest
};

pub struct AetherDebugService {
    session: Arc<SessionHandle>,
}

impl AetherDebugService {
    pub fn new(session: Arc<SessionHandle>) -> Self {
        Self { session }
    }

    async fn wait_for_match<F>(&self, rx: &mut broadcast::Receiver<CoreDebugEvent>, matcher: F) -> Result<CoreDebugEvent, Status>
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
    type SubscribeEventsStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<DebugEvent, Status>> + Send + Sync>>;

    async fn halt(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::Halt)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn resume(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::Resume)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn step(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::Step)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn step_over(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::StepOver)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn step_into(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::StepInto)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn step_out(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::StepOut)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn reset(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::Reset)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn get_status(&self, _request: Request<Empty>) -> Result<Response<StatusResponse>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::PollStatus)
             .map_err(|e| Status::internal(e.to_string()))?;

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
                        core_status = format!("{:?}", s);
                        received_status = true;
                        // If not halted, we are done (no PC). If halted, we might want to wait for PC if not received yet.
                        if !halted { break; }
                        if pc != 0 { break; }
                    }
                    CoreDebugEvent::Halted { pc: p } => {
                        pc = p;
                        halted = true;
                        if received_status { break; }
                    }
                    _ => {}
                }
            }
        }).await;

        Ok(Response::new(StatusResponse {
            halted,
            pc,
            core_status,
        }))
    }

    async fn read_memory(&self, _request: Request<ReadMemoryRequest>) -> Result<Response<ReadMemoryResponse>, Status> {
        Err(Status::unimplemented("Synchronous memory read not supported yet"))
    }

    async fn read_register(&self, _request: Request<ReadRegisterRequest>) -> Result<Response<ReadRegisterResponse>, Status> {
        Err(Status::unimplemented("Synchronous register read not supported yet"))
    }

    async fn write_memory(&self, _request: Request<WriteMemoryRequest>) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("WriteMemory not implemented"))
    }

    async fn write_register(&self, _request: Request<WriteRegisterRequest>) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("WriteRegister not implemented"))
    }

    async fn load_svd(&self, _request: Request<FileRequest>) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("LoadSvd not implemented"))
    }

    async fn get_peripherals(&self, _request: Request<Empty>) -> Result<Response<PeripheralResponse>, Status> {
        Err(Status::unimplemented("GetPeripherals not implemented"))
    }

    async fn read_peripheral(&self, _request: Request<PeripheralRequest>) -> Result<Response<proto::RegisterList>, Status> {
        Err(Status::unimplemented("ReadPeripheral not implemented"))
    }

    async fn write_peripheral(&self, _request: Request<PeripheralWriteRequest>) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("WritePeripheral not implemented"))
    }

    async fn set_breakpoint(&self, _request: Request<BreakpointRequest>) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("SetBreakpoint not implemented"))
    }

    async fn clear_breakpoint(&self, _request: Request<BreakpointRequest>) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("ClearBreakpoint not implemented"))
    }

    async fn list_breakpoints(&self, _request: Request<Empty>) -> Result<Response<BreakpointList>, Status> {
        Err(Status::unimplemented("ListBreakpoints not implemented"))
    }

    async fn watch_variable(&self, _request: Request<WatchVariableRequest>) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("WatchVariable not implemented"))
    }

    async fn rtt_write(&self, _request: Request<RttWriteRequest>) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("RttWrite not implemented"))
    }

    async fn get_tasks(&self, _request: Request<Empty>) -> Result<Response<TasksEvent>, Status> {
        Err(Status::unimplemented("GetTasks not implemented"))
    }

    async fn get_stack(&self, _request: Request<Empty>) -> Result<Response<StackResponse>, Status> {
        Err(Status::unimplemented("GetStack not implemented"))
    }

    async fn load_symbols(&self, request: Request<FileRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session.send(DebugCommand::LoadSymbols(std::path::PathBuf::from(req.path)))
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    type FlashStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<FlashProgress, Status>> + Send + 'static>>;

    async fn flash(&self, request: Request<FileRequest>) -> Result<Response<Self::FlashStream>, Status> {
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
                    aether_core::DebugEvent::FlashStatus(s) => FlashProgress { status: s, progress: 0.0, done: false, error: "".to_string() },
                    aether_core::DebugEvent::FlashProgress(p) => FlashProgress { status: "Flashing".to_string(), progress: p, done: false, error: "".to_string() },
                    aether_core::DebugEvent::FlashDone => {
                        let _ = tx.send(Ok(FlashProgress { status: "Done".to_string(), progress: 1.0, done: true, error: "".to_string() })).await;
                        break;
                    },
                    aether_core::DebugEvent::Error(e) => {
                         let _ = tx.send(Ok(FlashProgress { status: "Error".to_string(), progress: 0.0, done: true, error: e })).await;
                         break;
                    },
                    _ => continue,
                };
                if tx.send(Ok(progress)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))))
    }

    async fn disassemble(&self, request: Request<DisasmRequest>) -> Result<Response<DisasmResponse>, Status> {
        let req = request.into_inner();
        // Disassembly is tricky because it returns via event usually.
        // We'll implemented a request-response pattern by waiting for the specific event.
        // This acts as a bridge.

        let mut session_rx = self.session.subscribe();
        self.session.send(DebugCommand::Disassemble(req.address, req.count as usize))
            .map_err(|e| Status::internal(e.to_string()))?;

        // Wait for response with timeout
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while let Ok(event) = session_rx.recv().await {
                if let aether_core::DebugEvent::Disassembly(lines) = event {
                    let instructions = lines.iter().map(|l| format!("0x{:08X}:  {}  {}", l.address, l.mnemonic, l.op_str)).collect();
                    return Ok(DisasmResponse { instructions });
                }
            }
            Err(Status::internal("Stream closed"))
        }).await;

        match result {
             Ok(Ok(resp)) => Ok(Response::new(resp)),
             Ok(Err(e)) => Err(e),
             Err(_) => Err(Status::deadline_exceeded("Disassembly timed out")),
        }
    }

    async fn enable_semihosting(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::EnableSemihosting)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn enable_itm(&self, request: Request<ItmConfig>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session.send(DebugCommand::EnableItm { baud_rate: req.baud_rate })
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn list_probes(&self, _request: Request<Empty>) -> Result<Response<ProbeList>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::ListProbes).map_err(|e| Status::internal(e.to_string()))?;

        let event = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Probes(_))).await?;

        if let CoreDebugEvent::Probes(probes) = event {
             let proto_probes = probes.into_iter().enumerate().map(|(i, p)| ProtoProbeInfo {
                 index: i as u32,
                 name: p.name(),
                 serial: p.serial_number.unwrap_or_default(),
             }).collect();
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

        self.session.send(DebugCommand::Attach {
            probe_index: req.probe_index as usize,
            chip: req.chip,
            protocol,
            under_reset: req.under_reset,
        }).map_err(|e| Status::internal(e.to_string()))?;

        let _ = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Attached(_))).await?;
        Ok(Response::new(Empty {}))
    }

    // --- Events ---

    async fn subscribe_events(&self, _request: Request<Empty>) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        let rx = self.session.subscribe();
        let stream = BroadcastStream::new(rx);

        let output = stream.filter_map(|res| {
            match res {
                Ok(core_event) => map_core_event_to_proto(core_event).map(|e| Ok(e)),
                Err(_) => None, // Lagged or missed events
            }
        });

        Ok(Response::new(Box::pin(output)))
    }
}

pub fn map_core_event_to_proto(event: CoreDebugEvent) -> Option<DebugEvent> {
    match event {
        CoreDebugEvent::Halted { pc } => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Halted(proto::HaltedEvent { pc }))
        }),
        CoreDebugEvent::Resumed => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Resumed(proto::ResumedEvent {}))
        }),
        CoreDebugEvent::MemoryData(address, data) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Memory(proto::MemoryEvent { address, data }))
        }),
        CoreDebugEvent::RegisterValue(address, value) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Register(proto::RegisterEvent {
                register: address as u32,
                value
            }))
        }),
        CoreDebugEvent::Tasks(tasks) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Tasks(proto::TasksEvent {
                tasks: tasks.into_iter().map(|t| proto::TaskInfo {
                    name: t.name,
                    priority: t.priority,
                    state: format!("{:?}", t.state),
                    stack_usage: t.stack_usage,
                    stack_size: t.stack_size,
                    handle: t.handle,
                    task_type: format!("{:?}", t.task_type),
                }).collect()
            }))
        }),
        CoreDebugEvent::TaskSwitch { from, to, timestamp } => Some(DebugEvent {
            event: Some(proto::debug_event::Event::TaskSwitch(proto::TaskSwitchEvent {
                from,
                to,
                timestamp,
            }))
        }),
        CoreDebugEvent::PlotData { name, timestamp, value } => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Plot(proto::PlotEvent {
                name,
                timestamp,
                value,
            }))
        }),
        CoreDebugEvent::RttData(channel, data) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Rtt(proto::RttEvent {
                channel: channel as u32,
                data,
            }))
        }),
        CoreDebugEvent::SemihostingOutput(output) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Semihosting(SemihostingEvent {
                output
            }))
        }),
        CoreDebugEvent::ItmPacket(data) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Itm(ItmEvent {
                data
            }))
        }),
        CoreDebugEvent::Probes(probes) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Probes(proto::ProbeList {
                probes: probes.into_iter().enumerate().map(|(i, p)| proto::ProbeInfo {
                    index: i as u32,
                    name: p.name(),
                    serial: p.serial_number.unwrap_or_default(),
                }).collect()
            }))
        }),
        CoreDebugEvent::Attached(info) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Attached(proto::TargetInfo {
                name: info.name,
                flash_size: info.flash_size,
                ram_size: info.ram_size,
                architecture: info.architecture,
            }))
        }),
        CoreDebugEvent::Status(s) => Some(DebugEvent {
            event: Some(proto::debug_event::Event::Status(proto::StatusResponse {
                halted: s.is_halted(),
                pc: 0,
                core_status: format!("{:?}", s),
            }))
        }),
        _ => None
    }
}

pub fn map_proto_event_to_core(event: DebugEvent) -> Option<CoreDebugEvent> {
    match event.event? {
        proto::debug_event::Event::Halted(h) => Some(CoreDebugEvent::Halted { pc: h.pc }),
        proto::debug_event::Event::Resumed(_) => Some(CoreDebugEvent::Resumed),
        proto::debug_event::Event::Memory(m) => Some(CoreDebugEvent::MemoryData(m.address, m.data)),
        proto::debug_event::Event::Register(r) => Some(CoreDebugEvent::RegisterValue(r.register as u16, r.value)),
        proto::debug_event::Event::Tasks(t) => Some(CoreDebugEvent::Tasks(t.tasks.into_iter().map(|ti| aether_core::TaskInfo {
            name: ti.name,
            priority: ti.priority,
            state: match ti.state.as_str() {
                "Running" => aether_core::TaskState::Running,
                "Blocked" => aether_core::TaskState::Blocked,
                "Ready" => aether_core::TaskState::Ready,
                _ => aether_core::TaskState::Ready,
            },
            stack_usage: ti.stack_usage,
            stack_size: ti.stack_size,
            handle: ti.handle,
            task_type: if ti.task_type == "Async" { aether_core::TaskType::Async } else { aether_core::TaskType::Thread },
        }).collect())),
        proto::debug_event::Event::TaskSwitch(ts) => Some(CoreDebugEvent::TaskSwitch {
            from: ts.from,
            to: ts.to,
            timestamp: ts.timestamp,
        }),
        proto::debug_event::Event::Plot(p) => Some(CoreDebugEvent::PlotData {
            name: p.name,
            timestamp: p.timestamp,
            value: p.value,
        }),
        proto::debug_event::Event::Rtt(r) => Some(CoreDebugEvent::RttData(r.channel as usize, r.data)),
        proto::debug_event::Event::Breakpoint(_) | proto::debug_event::Event::Variable(_) => None,
        proto::debug_event::Event::Semihosting(s) => Some(CoreDebugEvent::SemihostingOutput(s.output)),
        proto::debug_event::Event::Itm(i) => Some(CoreDebugEvent::ItmPacket(i.data)),
        proto::debug_event::Event::Probes(p) => Some(CoreDebugEvent::Probes(p.probes.into_iter().map(|pi| aether_core::ProbeInfo {
            vendor_id: 0,
            product_id: 0,
            serial_number: if pi.serial.is_empty() { None } else { Some(pi.serial) },
            probe_type: aether_core::ProbeType::Other,
        }).collect())),
        proto::debug_event::Event::Attached(i) => Some(CoreDebugEvent::Attached(aether_core::TargetInfo {
            name: i.name,
            flash_size: i.flash_size,
            ram_size: i.ram_size,
            architecture: i.architecture,
        })),
        proto::debug_event::Event::Status(_) => None,
    }
}

pub async fn run_server(session: Arc<SessionHandle>, host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", host, port).parse()?;
    let service = AetherDebugService::new(session);

    println!("Agent API Server listening on {}", addr);

    Server::builder()
        .add_service(AetherDebugServer::new(service))
        .serve(addr)
        .await?;

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
}
