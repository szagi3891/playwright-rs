use playwright_rs::{ConnectOverCdpOptions, Playwright, expect, protocol::BrowserContextOptions};
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

struct DockerContainer {
    id: String,
}

impl DockerContainer {
    fn new(image: &str, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let output = Command::new("docker")
            .args(["run", "-d", "--rm", "-p", &format!("{}:3000", port), image])
            .output()?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to start docker container: {}", err).into());
        }

        let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("Started docker container: {}", id);
        Ok(Self { id })
    }
}

impl Drop for DockerContainer {
    fn drop(&mut self) {
        println!("Stopping docker container: {}", self.id);
        let _ = Command::new("docker").args(["stop", &self.id]).output();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = 3000;
    let endpoint = format!("http://localhost:{}", port);
    let traces_dir = "trace-output";

    // Ensure traces directory exists
    std::fs::create_dir_all(traces_dir)?;

    // Start browserless container
    let _container = DockerContainer::new("ghcr.io/browserless/chromium", port)?;

    // Launch Playwright
    let playwright = Playwright::launch().await?;

    let mut browser = None;
    for i in 0..15 {
        println!(
            "Próba połączenia z CDP endpoint: {} (próba {}/15)",
            endpoint,
            i + 1
        );
        let options = ConnectOverCdpOptions::new().timeout(10000.0);
        match playwright
            .chromium()
            .connect_over_cdp(&endpoint, Some(options))
            .await
        {
            Ok(b) => {
                browser = Some(b);
                println!("Connected to browserless container via CDP!");
                break;
            }
            Err(e) => {
                println!("Czekamy na gotowość przeglądarki... ({})", e);
                sleep(Duration::from_millis(1000)).await;
            }
        }
    }

    let browser = browser.ok_or("Nie udało się połączyć z Playwright via CDP")?;
    println!("Połączenie nawiązane, wersja: {}", browser.version());

    // Ustaw opcje kontekstu z traces_dir
    let context_options = BrowserContextOptions::builder()
        .traces_dir(traces_dir.to_string())
        .build();
    let context = browser.new_context_with_options(context_options).await?;
    let page = context.new_page().await?;

    println!("Przechodzę do https://www.twoup.agency");
    page.goto("https://www.twoup.agency", None).await?;
    let title = page.title().await?;
    println!("Tytuł strony: {}", title);
    let _ = page.screenshot(None).await;

    // Przykład użycia expect API
    // let h1 = page.locator("h1").await;
    // match expect(h1).to_be_visible().await {
    //     Ok(_) => println!("H1 jest widoczny!"),
    //     Err(e) => println!("H1 nie jest widoczny: {}", e),
    // }

    context.close().await?;
    browser.close().await?;
    // Po zakończeniu pracy trace zostanie zapisany w katalogu trace-output
    println!(
        "Katalog trace znajduje się w: {}. Możesz go otworzyć przez https://trace.playwright.dev/ lub polecenie: npx playwright show-trace {}/trace.zip",
        traces_dir, traces_dir
    );
    Ok(())
}
