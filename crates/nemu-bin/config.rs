#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub light: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { light: false }
    }
}
