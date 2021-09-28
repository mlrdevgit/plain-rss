use plain_rss::Config;
use std::process;

fn parse_config(args: &[String]) -> Result<Config, &str> {
    if args.len() < 3 {
        return Err("Not enough arguments; db path and opml import path expected");
    }

    let db_path = args[1].clone();
    let opml_path = args[2].clone();

    Ok(Config { db_path, opml_path })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config = parse_config(&args).unwrap_or_else(|e| {
        println!("{}", e);
        process::exit(1);
    });
    if let Err(e) = plain_rss::run(config) {
        println!("{}", e);
        process::exit(1);
    }
    process::exit(0);
}
