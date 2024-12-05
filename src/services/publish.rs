use crate::domain::Manifest;
use crate::error::{GameError, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tracing::info;

pub struct PublishService {
    client: Client,
    username: String,
    repo: String,
}

impl PublishService {
    pub fn new(username: String, repo: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            username,
            repo,
        }
    }

    pub async fn prepare(&self, manifest_path: &Path) -> Result<()> {
        // Create prepare directory
        let prepare_dir = Path::new("public");
        let images_dir = prepare_dir.join("images");
        tokio::fs::create_dir_all(&prepare_dir).await?;
        tokio::fs::create_dir_all(&images_dir).await?;

        // Read manifest
        info!("Reading manifest from {:?}", manifest_path);
        let manifest_content = tokio::fs::read_to_string(manifest_path).await?;
        let mut manifest: Manifest = serde_json::from_str(&manifest_content)?;

        let pb = ProgressBar::new(manifest.games.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                .map_err(|e| GameError::Other(e.to_string()))?,
        );

        // Process each game
        for game in &mut manifest.games {
            if let Some(ref url) = game.header_image {
                let filename = self.sanitize_filename(&game.title);
                let image_path = images_dir.join(format!("{}.jpg", filename));

                pb.set_message(format!("Processing {}", game.title));

                // Download image if it doesn't exist
                if !image_path.exists() {
                    if let Err(e) =  self.download_image(url, &image_path).await {
                        info!("Failed to download image for {}: {}", game.title, e);
                        continue;
                    }
                }

                game.header_image = Some(format!(
                    "https://{}.github.io/{}/images/{}.jpg",
                    self.username, self.repo, filename
                ));
            }
            pb.inc(1);
        }

        pb.finish_with_message("Done processing images!");

        // Save updated manifest
        let new_manifest_path = prepare_dir.join("manifest.json");
        let manifest_content = serde_json::to_string_pretty(&manifest)?;
        tokio::fs::write(new_manifest_path, manifest_content).await?;
        info!("Saved prepared manifest");

        Ok(())
    }

    async fn download_image(&self, url: &str, path: &Path) -> Result<()> {
        let response = self.client.get(url).send().await?;
        let bytes = response.bytes().await?;

        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(&bytes).await?;

        Ok(())
    }

    fn sanitize_filename(&self, title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| match c {
                ' ' | '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c if c.is_alphanumeric() || c == '-' || c == '_' => c,
                _ => '_',
            })
            .collect()
    }
}
