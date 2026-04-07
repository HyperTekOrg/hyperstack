//! `hs-mcp` — MCP server wrapping HyperStack streams for AI agent integration.
//!
//! See HYP-189 and `mcp_server_plan.md` at the repo root for the design.
//! This file is the v1 skeleton: stdio transport, single `ping` tool to
//! validate the rmcp handshake against Claude Desktop and other MCP clients.

use rmcp::{
    handler::server::router::tool::ToolRouter, model::*, tool, tool_handler, tool_router,
    transport::stdio, ErrorData as McpError, ServerHandler, ServiceExt,
};

#[derive(Clone)]
pub struct HyperstackMcp {
    tool_router: ToolRouter<HyperstackMcp>,
}

#[tool_router]
impl HyperstackMcp {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Health check. Returns \"pong\" if the server is alive.")]
    async fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text("pong")]))
    }
}

impl Default for HyperstackMcp {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler]
impl ServerHandler for HyperstackMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logs go to stderr so they don't pollute the stdio MCP transport on stdout.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("starting hs-mcp stdio server");
    let service = HyperstackMcp::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
