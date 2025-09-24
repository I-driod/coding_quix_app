
#[derive(Debug, Clone)]
pub struct Config{
    pub mongodb_uri: String,
    pub jwt_secret: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        let mongodb_uri = std::env::var("MONGODB_URI").expect("MONGODB_URI must be set");
        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8000".to_string())
            .parse()
            .expect("PORT must be a valid u16");

        Config {
            mongodb_uri,
            jwt_secret,
            port,
        }
    }
}