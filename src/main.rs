use playwright_rs::Playwright;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

struct DockerContainer {
    id: String,
}

impl DockerContainer {
    fn new(image: &str, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let output = Command::new("docker")
            .args([
                "run",
                "-d",
                "--rm",
                "-p",
                &format!("{}:{}", port, port),
                image,
                "npx",
                "-y",
                "playwright@1.56.1",
                "run-server",
                "--port",
                &port.to_string(),
                "--path",
                "/playwright",
            ])
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

    // W nowym wątku (lub po prostu w tle demona) odpalamy kontener
    let _container = DockerContainer::new("mcr.microsoft.com/playwright:v1.56.1-jammy", port)?;

    let playwright = Playwright::launch().await?;

    let mut browser = None;
    for i in 0..15 {
        // Gdy ten playwright będzie gotowy, łączymy się
        match playwright
            .chromium()
            .connect(&format!("ws://localhost:{}/playwright", port), None) //TODO - brakująca metoda connect_over_cdp
            .await
        {
            Ok(b) => {
                browser = Some(b);
                println!("Connected to browserless container!");
                break;
            }
            Err(e) => {
                println!(
                    "Czekamy na gotowość przeglądarki... próba {}/15 ({})",
                    i + 1,
                    e
                );
                sleep(Duration::from_millis(1000)).await;
            }
        }
    }

    let browser = browser.ok_or("Nie udało się połączyć z Playwright")?;

    println!("Połączenie nawiązane, przechodzę do dalszej części testu !!!!!!!!!!!!");

    // Odpalalszą część testu
    let page = browser.new_page().await?;

    page.goto("https://www.twoup.agency", None).await?;
    let title = page.title().await?;
    println!("Tytuł strony: {}", title);

    browser.close().await?;
    // Po zakończeniu pracy zmienna `_container` wypada ze Scope'u i uruchamia "zwiń kontener" w `drop`.
    Ok(())
}

//https://www.twoup.agency
