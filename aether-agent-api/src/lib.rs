use tonic::{transport::Server, Request, Response, Status};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use aether_core::{SessionHandle, DebugCommand, DebugEvent as CoreDebugEvent};
use std::sync::Arc;

pub mod proto {
    tonic::include_proto!("aether");
}

use proto::aether_debug_server::{AetherDebug, AetherDebugServer};
use proto::{Empty, StatusResponse, ReadMemoryRequest, ReadMemoryResponse, ReadRegisterRequest, ReadRegisterResponse, DebugEvent};

pub struct AetherDebugService {
    session: Arc<SessionHandle>,
}

impl AetherDebugService {
    pub fn new(session: Arc<SessionHandle>) -> Self {
        Self { session }
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

    async fn reset(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        // Reset not yet implemented in DebugCommand, skipping for now
        Err(Status::unimplemented("Reset not implemented"))
    }

    async fn get_status(&self, _request: Request<Empty>) -> Result<Response<StatusResponse>, Status> {
        self.session.send(DebugCommand::PollStatus)
             .map_err(|e| Status::internal(e.to_string()))?;

        // Return a dummy response for now, real status comes via events
        Ok(Response::new(StatusResponse {
            halted: false,
            pc: 0,
            core_status: "Unknown".to_string(),
        }))
    }

    async fn read_memory(&self, _request: Request<ReadMemoryRequest>) -> Result<Response<ReadMemoryResponse>, Status> {
        Err(Status::unimplemented("Synchronous memory read not supported yet"))
    }

    async fn read_register(&self, _request: Request<ReadRegisterRequest>) -> Result<Response<ReadRegisterResponse>, Status> {
        Err(Status::unimplemented("Synchronous register read not supported yet"))
    }

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
