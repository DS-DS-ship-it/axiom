use tokio::{io::{AsyncBufReadExt, BufReader}, net::TcpListener};
use tracing::{info, warn};

use crate::error::Result;

pub async fn run_listener(bind_addr: &str) -> Result<()> {
    let listener = TcpListener::bind(bind_addr).await?;
    info!(%bind_addr, "p2p listener started");

    loop {
        let (stream, peer) = listener.accept().await?;
        info!(%peer, "peer connected");
        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.len() > 1_000_000 {
                    warn!(%peer, "oversized message dropped");
                    break;
                }
                info!(%peer, message = %line, "received message");
            }
        });
    }
}
