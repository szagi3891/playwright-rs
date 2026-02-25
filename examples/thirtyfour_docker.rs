use std::process::Command;
use std::time::Duration;
use thirtyfour::prelude::*;
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
                &format!("{}:4444", port), // Selenium standalone domyślnie używa portu 4444 dla WebDriver
                image,
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
    let port = 4444;

    // Odpalamy kontener Selenium z wbudowanym Chrome (WebDriver i przeglądarka w jednym)
    let _container = DockerContainer::new("seleniarm/standalone-chromium:latest", port)?;

    // WebDriver / Selenium Standalone potrzebuje chwili na podniesienie serwera w Javie
    let mut driver = None;
    let url = format!("http://localhost:{}", port);

    // Próbujemy połączyć się z serwerem WebDriver (Selenium) max 15 razy (czyli ok. 15 sekund)
    for i in 0..15 {
        println!(
            "Czekamy na gotowość Selenium (WebDriver)... próba {}/15",
            i + 1
        );

        let caps = DesiredCapabilities::chrome();
        match WebDriver::new(&url, caps).await {
            Ok(d) => {
                driver = Some(d);
                println!("Connected to Selenium Standalone!");
                break;
            }
            Err(_) => {
                sleep(Duration::from_millis(1000)).await;
            }
        }
    }

    let driver = driver.ok_or("Nie udało się połączyć z WebDriver")?;

    println!("Połączenie nawiązane, przechodzę do dalszej części testu !!!!!!!!!!!!");

    // Odpalamy dalszą część testu
    driver.goto("https://www.twoup.agency").await?;

    let title = driver.title().await?;
    println!("Tytuł strony: {}", title);

    // Na koniec zamykamy sesję w WebDriverze
    driver.quit().await?;

    // Po zakończeniu pracy zmienna `_container` wypada ze Scope'u i uruchamia "zwiń kontener" w `drop`.
    Ok(())
}
