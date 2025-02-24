use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct DbConfig {
    #[arg(long)]
    pub original_user: String,
    #[arg(long)]
    pub original_password: String,
    #[arg(long)]
    pub original_host: String,
    #[arg(long, default_value = "3306")]
    pub original_port: String,
    #[arg(long)]
    pub original_schema: String,

    #[arg(long)]
    pub target_user: String,
    #[arg(long)]
    pub target_password: String,
    #[arg(long)]
    pub target_host: String,
    #[arg(long, default_value = "3306")]
    pub target_port: String,
    #[arg(long)]
    pub target_schema: String,
}
