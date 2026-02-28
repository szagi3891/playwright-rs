use playwright_rs::{ConnectOverCdpOptions, Playwright, expect};
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

    // W nowym wątku (lub po prostu w tle demona) odpalamy kontener (browserless)
    let _container = DockerContainer::new("ghcr.io/browserless/chromium", port)?;

    // Launch local Playwright driver (required for protocol management)
    let playwright = Playwright::launch().await?;

    let mut browser = None;
    for i in 0..15 {
        println!(
            "Próba połączenia z CDP endpoint: {} (próba {}/15)",
            endpoint,
            i + 1
        );

        // Connect to the remote Chrome via CDP (Chromium only)
        let options = ConnectOverCdpOptions::new().timeout(10000.0); // 10s timeout
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

    // Odpalalszą część testu
    let page = browser.new_page().await?;

    println!("Przechodzę do https://www.twoup.agency");
    page.goto("https://www.twoup.agency", None).await?;

    let title = page.title().await?;
    println!("Tytuł strony: {}", title);

    let aaa = page.screenshot(None).await;

    // // Przykład użycia expect API (np. sprawdzenie czy nagłówek H1 istnieje)
    // let h1 = page.locator("h1").await;
    // match expect(h1).to_be_visible().await {
    //     Ok(_) => println!("H1 jest widoczny!"),
    //     Err(e) => println!("H1 nie jest widoczny: {}", e),
    // }

    browser.close().await?;
    // Po zakończeniu pracy zmienna `_container` wypada ze Scope'u i uruchamia "zwiń kontener" w `drop`.
    Ok(())
}
