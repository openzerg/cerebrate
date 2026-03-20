use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::AppState;
use crate::grpc::cerebrate::*;
use crate::grpc::cerebrate::swarm_service_server::SwarmService;

pub struct SwarmGrpcServer {
    state: Arc<AppState>,
}

impl SwarmGrpcServer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl SwarmService for SwarmGrpcServer {
    async fn list_agents(&self, _request: Request<Empty>) -> Result<Response<AgentListResponse>, Status> {
        Ok(Response::new(AgentListResponse { agents: vec![] }))
    }

    async fn get_agent(&self, _request: Request<GetAgentRequest>) -> Result<Response<AgentInfo>, Status> {
        Err(Status::not_found("Agent not found"))
    }

    async fn create_agent(&self, _request: Request<CreateAgentRequest>) -> Result<Response<AgentInfo>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn delete_agent(&self, _request: Request<DeleteAgentRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn enable_agent(&self, _request: Request<EnableAgentRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn disable_agent(&self, _request: Request<DisableAgentRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn bind_model(&self, _request: Request<BindModelRequest>) -> Result<Response<AgentInfo>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn unbind_model(&self, _request: Request<UnbindModelRequest>) -> Result<Response<AgentInfo>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn list_providers(&self, _request: Request<Empty>) -> Result<Response<ProviderListResponse>, Status> {
        Ok(Response::new(ProviderListResponse { providers: vec![] }))
    }

    async fn create_provider(&self, _request: Request<CreateProviderRequest>) -> Result<Response<ProviderInfo>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn delete_provider(&self, _request: Request<DeleteProviderRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn enable_provider(&self, _request: Request<EnableProviderRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn disable_provider(&self, _request: Request<DisableProviderRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn list_models(&self, _request: Request<Empty>) -> Result<Response<ModelListResponse>, Status> {
        Ok(Response::new(ModelListResponse { models: vec![] }))
    }

    async fn create_model(&self, _request: Request<CreateModelRequest>) -> Result<Response<ModelInfo>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn delete_model(&self, _request: Request<DeleteModelRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn enable_model(&self, _request: Request<EnableModelRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn disable_model(&self, _request: Request<DisableModelRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn create_checkpoint(&self, _request: Request<CreateCheckpointRequest>) -> Result<Response<CheckpointInfo>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn list_checkpoints(&self, _request: Request<ListCheckpointsRequest>) -> Result<Response<CheckpointListResponse>, Status> {
        Ok(Response::new(CheckpointListResponse { checkpoints: vec![] }))
    }

    async fn delete_checkpoint(&self, _request: Request<DeleteCheckpointRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn rollback_checkpoint(&self, _request: Request<RollbackCheckpointRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn clone_checkpoint(&self, _request: Request<CloneCheckpointRequest>) -> Result<Response<AgentInfo>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn list_skills(&self, _request: Request<Empty>) -> Result<Response<SkillListResponse>, Status> {
        Ok(Response::new(SkillListResponse { skills: vec![] }))
    }

    async fn get_skill(&self, _request: Request<GetSkillRequest>) -> Result<Response<SkillInfo>, Status> {
        Err(Status::not_found("Skill not found"))
    }

    async fn clone_skill(&self, _request: Request<CloneSkillRequest>) -> Result<Response<SkillInfo>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn pull_skill(&self, _request: Request<PullSkillRequest>) -> Result<Response<SkillInfo>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn delete_skill(&self, _request: Request<DeleteSkillRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn list_tools(&self, _request: Request<Empty>) -> Result<Response<ToolListResponse>, Status> {
        Ok(Response::new(ToolListResponse { tools: vec![] }))
    }

    async fn get_tool(&self, _request: Request<GetToolRequest>) -> Result<Response<ToolInfo>, Status> {
        Err(Status::not_found("Tool not found"))
    }

    async fn clone_tool(&self, _request: Request<CloneToolRequest>) -> Result<Response<ToolInfo>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn pull_tool(&self, _request: Request<PullToolRequest>) -> Result<Response<ToolInfo>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }

    async fn delete_tool(&self, _request: Request<DeleteToolRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn authorize_tool(&self, _request: Request<AuthorizeToolRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn revoke_tool(&self, _request: Request<RevokeToolRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn invoke_tool(&self, _request: Request<InvokeToolRequest>) -> Result<Response<InvokeToolResponse>, Status> {
        Ok(Response::new(InvokeToolResponse { output_json: "{}".to_string() }))
    }

    async fn list_tool_env(&self, _request: Request<ListToolEnvRequest>) -> Result<Response<ToolEnvListResponse>, Status> {
        Ok(Response::new(ToolEnvListResponse { keys: vec![] }))
    }

    async fn set_tool_env(&self, _request: Request<SetToolEnvRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn delete_tool_env(&self, _request: Request<DeleteToolEnvRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn get_stats(&self, _request: Request<Empty>) -> Result<Response<StatsResponse>, Status> {
        Ok(Response::new(StatsResponse {
            total_agents: 0,
            online_agents: 0,
            enabled_agents: 0,
        }))
    }

    type SubscribeEventsStream = ReceiverStream<Result<AgentEvent, Status>>;

    async fn subscribe_events(&self, _request: Request<SubscribeEventsRequest>) -> Result<Response<ReceiverStream<Result<AgentEvent, Status>>>, Status> {
        let (tx, rx) = mpsc::channel(128);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn list_sessions(&self, _request: Request<ListSessionsRequest>) -> Result<Response<SessionListResponse>, Status> {
        Ok(Response::new(SessionListResponse { sessions: vec![], total: 0 }))
    }

    async fn get_session(&self, _request: Request<GetSessionRequest>) -> Result<Response<SessionInfo>, Status> {
        Err(Status::not_found("Session not found"))
    }

    async fn get_session_messages(&self, _request: Request<GetSessionMessagesRequest>) -> Result<Response<MessageListResponse>, Status> {
        Ok(Response::new(MessageListResponse { messages: vec![], total: 0 }))
    }

    async fn send_session_chat(&self, _request: Request<SendSessionChatRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn interrupt_session(&self, _request: Request<InterruptSessionRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn get_session_context(&self, _request: Request<GetSessionContextRequest>) -> Result<Response<SessionContextResponse>, Status> {
        Ok(Response::new(SessionContextResponse { context_json: "{}".to_string() }))
    }

    async fn list_processes(&self, _request: Request<ListProcessesRequest>) -> Result<Response<ProcessListResponse>, Status> {
        Ok(Response::new(ProcessListResponse { processes: vec![], total: 0 }))
    }

    async fn get_process(&self, _request: Request<GetProcessRequest>) -> Result<Response<ProcessInfo>, Status> {
        Err(Status::not_found("Process not found"))
    }

    async fn get_process_output(&self, _request: Request<GetProcessOutputRequest>) -> Result<Response<ProcessOutputResponse>, Status> {
        Ok(Response::new(ProcessOutputResponse { content: "".to_string(), total_size: 0 }))
    }

    async fn list_tasks(&self, _request: Request<ListTasksRequest>) -> Result<Response<TaskListResponse>, Status> {
        Ok(Response::new(TaskListResponse { tasks: vec![], total: 0 }))
    }

    async fn get_task(&self, _request: Request<GetTaskRequest>) -> Result<Response<TaskInfo>, Status> {
        Err(Status::not_found("Task not found"))
    }

    async fn list_activities(&self, _request: Request<ListActivitiesRequest>) -> Result<Response<ActivityListResponse>, Status> {
        Ok(Response::new(ActivityListResponse { activities: vec![], total: 0 }))
    }

    async fn send_message(&self, _request: Request<SendMessageRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn send_remind(&self, _request: Request<SendRemindRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn list_builtin_tools(&self, _request: Request<ListBuiltinToolsRequest>) -> Result<Response<BuiltinToolListResponse>, Status> {
        Ok(Response::new(BuiltinToolListResponse { tools: vec![] }))
    }

    async fn execute_builtin_tool(&self, _request: Request<ExecuteBuiltinToolRequest>) -> Result<Response<ExecuteBuiltinToolResponse>, Status> {
        Ok(Response::new(ExecuteBuiltinToolResponse {
            title: "".to_string(),
            output: "".to_string(),
            metadata_json: "{}".to_string(),
            attachments_json: "[]".to_string(),
            truncated: false,
        }))
    }
}