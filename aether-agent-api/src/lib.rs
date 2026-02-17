#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::option_if_let_else)]

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
    BreakpointRequest, BreakpointList, StackResponse, StackFrame,
    TasksEvent, TaskInfo, PeripheralRequest, PeripheralResponse, PeripheralWriteRequest,
    WatchVariableRequest, RttWriteRequest, DebugEvent,
    FileRequest, FlashProgress, DisasmRequest, DisasmResponse,
    ItmConfig, SemihostingEvent, ItmEvent
};

pub struct AetherDebugService {
    session: Arc<SessionHandle>,
}

impl AetherDebugService {
    #[must_use] 
    pub const fn new(session: Arc<SessionHandle>) -> Self {
        Self { session }
    }

    async fn wait_for_match<F>(&self, rx: &mut broadcast::Receiver<CoreDebugEvent>, matcher: F) -> Result<CoreDebugEvent, Status>
    where
        F: Fn(&CoreDebugEvent) -> bool + Send + 'static,
    {
        let timeout = Duration::from_secs(2);
        let start = std::time::Instant::now();
        
        loop {
            if start.elapsed() > timeout {
                return Err(Status::deadline_exceeded("Timeout waiting for debug event"));
            }
            
            match rx.recv().await {
                Ok(event) => {
                    if matcher(&event) {
                        return Ok(event);
                    }
                    if let CoreDebugEvent::Error(_e) = event {
                        // Log error but continue waiting? OR fail?
                        // For now just continue implementation as per previous plan
                    }
                }
                Err(_) => return Err(Status::internal("Event stream lagged or closed")),
            }
        }
    }
}

#[tonic::async_trait]
impl AetherDebug for AetherDebugService {
    type SubscribeEventsStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<DebugEvent, Status>> + Send + Sync>>;

    // --- Execution Control ---

    async fn halt(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::Halt).map_err(|e| Status::internal(e.to_string()))?;
        let _ = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Halted { .. })).await?;
        Ok(Response::new(Empty {}))
    }

    async fn resume(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.session.send(DebugCommand::Resume).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn step(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::Step).map_err(|e| Status::internal(e.to_string()))?;
        let _ = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Halted { .. })).await?;
        Ok(Response::new(Empty {}))
    }

    async fn step_over(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::StepOver).map_err(|e| Status::internal(e.to_string()))?;
        let _ = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Halted { .. })).await?;
        Ok(Response::new(Empty {}))
    }

    async fn step_into(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::StepInto).map_err(|e| Status::internal(e.to_string()))?;
        let _ = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Halted { .. })).await?;
        Ok(Response::new(Empty {}))
    }

    async fn step_out(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::StepOut).map_err(|e| Status::internal(e.to_string()))?;
        let _ = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Halted { .. })).await?;
        Ok(Response::new(Empty {}))
    }

    async fn reset(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::Reset).map_err(|e| Status::internal(e.to_string()))?;
        let _ = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Halted { .. })).await?;
        Ok(Response::new(Empty {}))
    }

    // --- Breakpoints ---

    async fn set_breakpoint(&self, request: Request<BreakpointRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session.send(DebugCommand::SetBreakpoint(req.address))
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn clear_breakpoint(&self, request: Request<BreakpointRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session.send(DebugCommand::ClearBreakpoint(req.address))
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn list_breakpoints(&self, _request: Request<Empty>) -> Result<Response<BreakpointList>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::ListBreakpoints).map_err(|e| Status::internal(e.to_string()))?;
        
        let event = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Breakpoints(_))).await?;
        
        if let CoreDebugEvent::Breakpoints(addrs) = event {
             Ok(Response::new(BreakpointList { addresses: addrs }))
        } else {
             Err(Status::internal("Unexpected event"))
        }
    }

    // --- State Inspection ---

    async fn get_status(&self, _request: Request<Empty>) -> Result<Response<StatusResponse>, Status> {
        self.session.send(DebugCommand::PollStatus).map_err(|e| Status::internal(e.to_string()))?;
        // Return mostly dummy or last known status.
        Ok(Response::new(StatusResponse {
            halted: false, 
            pc: 0,
            core_status: "Only via events".to_string(),
        }))
    }

    async fn read_memory(&self, request: Request<ReadMemoryRequest>) -> Result<Response<ReadMemoryResponse>, Status> {
        let req = request.into_inner();
        let mut rx = self.session.subscribe();
        
        self.session.send(DebugCommand::ReadMemory(req.address, req.length as usize))
            .map_err(|e| Status::internal(e.to_string()))?;
            
        let event = self.wait_for_match(&mut rx, move |e| {
            if let CoreDebugEvent::MemoryData(addr, _) = e {
                *addr == req.address
            } else {
                false
            }
        }).await?;
        
        if let CoreDebugEvent::MemoryData(_, data) = event {
            Ok(Response::new(ReadMemoryResponse { data }))
        } else {
            Err(Status::internal("Unexpected event"))
        }
    }
    
    async fn write_memory(&self, request: Request<WriteMemoryRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session.send(DebugCommand::WriteMemory(req.address, req.data))
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn read_register(&self, request: Request<ReadRegisterRequest>) -> Result<Response<ReadRegisterResponse>, Status> {
        let req = request.into_inner();
        let mut rx = self.session.subscribe();
        
        self.session.send(DebugCommand::ReadRegister(req.register_number as u16))
            .map_err(|e| Status::internal(e.to_string()))?;
            
        let event = self.wait_for_match(&mut rx, move |e| {
            if let CoreDebugEvent::RegisterValue(reg, _) = e {
                *reg == req.register_number as u16
            } else {
                false
            }
        }).await?;
        
        if let CoreDebugEvent::RegisterValue(_, val) = event {
            Ok(Response::new(ReadRegisterResponse { value: val }))
        } else {
             Err(Status::internal("Unexpected event"))
        }
    }

    async fn write_register(&self, request: Request<WriteRegisterRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session.send(DebugCommand::WriteRegister(req.register_number as u16, req.value))
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }
    
    // --- High Level ---
    
    async fn get_stack(&self, _request: Request<Empty>) -> Result<Response<StackResponse>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::GetStack).map_err(|e| Status::internal(e.to_string()))?;
        
        let event = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Stack(_))).await?;
        
        if let CoreDebugEvent::Stack(frames) = event {
            let proto_frames = frames.into_iter().map(|f| StackFrame {
                pc: u64::from(f.pc),
                function_name: f.function_name,
                file: f.source_file.unwrap_or_default(),
                line: f.line.unwrap_or(0) as u32,
            }).collect();
            Ok(Response::new(StackResponse { frames: proto_frames }))
        } else {
            Err(Status::internal("Unexpected event"))
        }
    }
    
    async fn get_tasks(&self, _request: Request<Empty>) -> Result<Response<TasksEvent>, Status> {
        let mut rx = self.session.subscribe();
        self.session.send(DebugCommand::GetTasks).map_err(|e| Status::internal(e.to_string()))?;
        
        let event = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Tasks(_))).await?;
        
        if let CoreDebugEvent::Tasks(tasks) = event {
             let proto_tasks = tasks.into_iter().map(|t| TaskInfo {
                 name: t.name,
                 priority: t.priority,
                 state: format!("{:?}", t.state),
                 stack_usage: t.stack_usage,
                 stack_size: t.stack_size,
                 handle: t.handle,
                 task_type: format!("{:?}", t.task_type),
             }).collect();
             Ok(Response::new(TasksEvent { tasks: proto_tasks }))
        } else {
             Err(Status::internal("Unexpected event"))
        }
    }
    
    async fn watch_variable(&self, request: Request<WatchVariableRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session.send(DebugCommand::WatchVariable(req.name))
             .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }
    
    // --- Peripherals ---
    
    async fn read_peripheral(&self, request: Request<PeripheralRequest>) -> Result<Response<PeripheralResponse>, Status> {
         let req = request.into_inner();
         let mut rx = self.session.subscribe();
         
         self.session.send(DebugCommand::ReadPeripheralValues(req.peripheral.clone()))
             .map_err(|e| Status::internal(e.to_string()))?;
             
         let event = self.wait_for_match(&mut rx, |e| matches!(e, CoreDebugEvent::Registers(_))).await?;
         
         if let CoreDebugEvent::Registers(regs) = event {
             if let Some(r) = regs.iter().find(|r| r.name == req.register) {
                  return Ok(Response::new(PeripheralResponse { value: r.value.unwrap_or(0) }));
             }
             Err(Status::not_found("Register not found"))
         } else {
             Err(Status::internal("Unexpected event"))
         }
    }
    
    async fn write_peripheral(&self, request: Request<PeripheralWriteRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session.send(DebugCommand::WritePeripheralField {
            peripheral: req.peripheral,
            register: req.register,
            field: req.field,
            value: req.value
        }).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    // --- RTT ---
    
    async fn rtt_write(&self, request: Request<RttWriteRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session.send(DebugCommand::RttWrite {
            channel: req.channel as usize,
            data: req.data,
        }).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }


    async fn load_svd(&self, request: Request<FileRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.session.send(DebugCommand::LoadSvd(std::path::PathBuf::from(req.path)))
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
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
    
    // --- Events ---
    
    async fn subscribe_events(&self, _request: Request<Empty>) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        let rx = self.session.subscribe();
        let stream = BroadcastStream::new(rx);

        let output = stream.filter_map(|res| {
            match res {
                Ok(core_event) => map_core_event_to_proto(core_event).map(Ok),
                Err(_) => None, 
            }
        });

        Ok(Response::new(Box::pin(output)))
    }
}

#[must_use] 
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
                register: u32::from(address), 
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
        _ => None
    }
}

#[must_use] 
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
    }
}

pub async fn run_server(session: Arc<SessionHandle>, host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{host}:{port}").parse()?;
    let service = AetherDebugService::new(session);

    println!("Agent API Server listening on {addr}");

    Server::builder()
        .add_service(AetherDebugServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
