mod handler;
mod client;
mod interceptor;

pub use handler::SwarmGrpcServer;
pub use client::AgentGrpcClient;
pub use interceptor::AuthInterceptor;

pub mod cerebrate {
    tonic::include_proto!("swarm");
}