#[tokio::main]
async fn main() -> anyhow::Result<()> {
    raspi_dashboard_backend::run_server().await
}
