fn get_env_var(key: &str, default: Option<&str>) -> String {
    std::env::var(key).unwrap_or_else(|_| {
        default
            .map(|s| s.to_string())
            .unwrap_or_else(|| panic!("{} must be set", key))
    })
}

#[derive(Debug, Clone)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
}

impl Config {
    pub(crate) fn new() -> Self {
        Self {
            server_host: get_env_var("SERVER_HOST", Some("127.0.0.1")),
            server_port: get_env_var("SERVER_PORT", Some("1337"))
                .parse()
                .expect("SERVER_PORT must be a number"),
        }
    }
}
