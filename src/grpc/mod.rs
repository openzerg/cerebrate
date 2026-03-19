mod handler;
mod client;

pub use handler::SwarmGrpcServer;
pub use client::AgentGrpcClient;

pub mod swarm {
    tonic::include_proto!("swarm");
}