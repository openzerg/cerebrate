mod handler;
mod client;

pub use handler::SwarmGrpcServer;
pub use client::AgentGrpcClient;

pub mod cerebrate {
    tonic::include_proto!("swarm");
}