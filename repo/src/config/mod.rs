use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub tg_api_id: i32,
    pub tg_api_hash: String,
    pub tg_session_file: String,
    pub tg_phone: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let tg_api_id = match env::var("TG_API_ID") {
            Ok(v) => match v.parse::<i32>() {
                Ok(id) => id,
                Err(_) => {
                    eprintln!("Invalid format for environment variable: TG_API_ID");
                    std::process::exit(1);
                }
            },
            Err(_) => {
                eprintln!("Missing required environment variable: TG_API_ID");
                std::process::exit(1);
            }
        };

        let tg_api_hash = match env::var("TG_API_HASH") {
            Ok(v) if !v.is_empty() => v,
            _ => {
                eprintln!("Missing required environment variable: TG_API_HASH");
                std::process::exit(1);
            }
        };

        let tg_session_file = match env::var("TG_SESSION_FILE") {
            Ok(v) if !v.is_empty() => v,
            _ => {
                eprintln!("Missing required environment variable: TG_SESSION_FILE");
                std::process::exit(1);
            }
        };

        let tg_phone = match env::var("TG_PHONE") {
            Ok(v) if !v.is_empty() => Some(v),
            _ => None,
        };

        Self {
            tg_api_id,
            tg_api_hash,
            tg_session_file,
            tg_phone,
        }
    }
}
