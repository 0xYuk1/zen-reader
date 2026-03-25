#[derive(Clone)]
pub struct AppState {
    pub komga: Option<KomgaConfig>,
    pub http: reqwest::Client,
    pub progress_path: std::path::PathBuf,
}

#[derive(Clone)]
pub struct KomgaConfig {
    pub url: String,
    pub user: String,
    pub password: String,
}

impl AppState {
    pub fn new() -> Self {
        let komga = match (
            std::env::var("KOMGA_URL"),
            std::env::var("KOMGA_USER"),
            std::env::var("KOMGA_PASSWORD"),
        ) {
            (Ok(url), Ok(user), Ok(password)) => Some(KomgaConfig {
                url: url.trim_end_matches('/').to_string(),
                user,
                password,
            }),
            _ => None,
        };

        let progress_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("zen-reader")
            .join("progress.json");

        Self {
            komga,
            http: reqwest::Client::new(),
            progress_path,
        }
    }

    pub fn komga(&self) -> Result<&KomgaConfig, &'static str> {
        self.komga
            .as_ref()
            .ok_or("Komga not configured. Set KOMGA_URL, KOMGA_USER, KOMGA_PASSWORD env vars.")
    }
}
