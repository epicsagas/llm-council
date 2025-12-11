mod mcp;
mod tools;
mod cli_runner;

use anyhow::Result;
use mcp::McpServer;

#[tokio::main]
async fn main() -> Result<()> {
    let mut server = McpServer::new();
    server.run().await
}

