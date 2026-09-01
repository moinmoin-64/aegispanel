use std::fs;
use std::process::Command;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("===============================================");
    info!(" AegisPanel OS Emergency Recovery Subsystem   ");
    info!("===============================================");

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    info!("Emergency Recovery HTTP Web Server listening on http://0.0.0.0:8080");

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Recovery client connected from: {}", addr);

        tokio::spawn(async move {
            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut request_line = String::new();

            if let Ok(_) = buf_reader.read_line(&mut request_line).await {
                let mut trigger_reboot = false;

                let response_body = if request_line.starts_with("POST /api/factory_reset") {
                    warn!("FACTORY RESET TRIGGERED! Wiping persistent config in /etc/aegispanel...");
                    let _ = fs::remove_dir_all("/etc/aegispanel");
                    let _ = fs::create_dir_all("/etc/aegispanel");
                    trigger_reboot = true;
                    r#"{"status":"ok","message":"Factory reset complete. Rebooting..."}"#
                } else if request_line.starts_with("POST /api/rollback") {
                    info!("ROLLBACK TRIGGERED! Toggling U-Boot active slot...");
                    let _ = Command::new("fw_setenv").arg("BOOT_TRY").arg("1").status();
                    trigger_reboot = true;
                    r#"{"status":"ok","message":"Rollback boot slot configured. Rebooting..."}"#
                } else if request_line.starts_with("POST /api/reboot") {
                    info!("REBOOT TRIGGERED!");
                    trigger_reboot = true;
                    r#"{"status":"ok","message":"Rebooting system..."}"#
                } else {
                    r#"{"status":"ok","subsystem":"AegisPanel OS Emergency Recovery v1.0.0","available_actions":["/api/factory_reset","/api/rollback","/api/reboot"]}"#
                };

                let http_response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );

                let _ = writer.write_all(http_response.as_bytes()).await;

                if trigger_reboot {
                    tokio::spawn(async {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        let _ = Command::new("reboot").status();
                    });
                }
            }
        });
    }
}
