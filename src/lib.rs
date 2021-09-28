extern crate feed_rs;
extern crate html2text;
extern crate opml;
extern crate reqwest;
extern crate sqlite;

// TODO: add https://docs.rs/tui/0.16.0/tui/ and https://docs.rs/crossterm/0.21.0/crossterm/
// https://docs.rs/html2text/0.2.1/html2text/
// https://docs.rs/sqlite/0.26.0/sqlite/index.html
// https://docs.rs/opml/1.1.2/opml/index.html
// https://docs.rs/feed-rs/1.0.0/feed_rs/

use std::error::Error;

fn text_content_or<'a>(text: &'a Option<feed_rs::model::Text>, or_text: &'a str) -> &'a str {
    if let Some(t) = text {
        return &t.content;
    }
    return or_text;
}

fn get_entry_content_and_url(entry: &feed_rs::model::Entry) -> (String, String) {
    for link in &entry.links {
        let response = reqwest::blocking::get(&link.href).unwrap();
        return (html2text::from_read(response, 80), link.href.clone());
    }
    return (String::new(), String::new());
}

fn get_entry_content(entry: &feed_rs::model::Entry) -> String {
    for link in &entry.links {
        let response = reqwest::blocking::get(&link.href).unwrap();
        return html2text::from_read(response, 80);
    }
    return String::new();
}

fn process_outline(title: &str, url: &str) {
    let response = reqwest::blocking::get(url);
    if let Err(e) = response {
        println!("  Failed to read '{}' from {}: {}", title, url, e);
        return;
    }
    let feed = feed_rs::parser::parse(response.unwrap());
    if let Err(e) = feed {
        println!("  Failed to parse '{}' from {}: {}", title, url, e);
        return;
    }
    for entry in &feed.unwrap().entries {
        println!("  {}", text_content_or(&entry.title, "(no title)"));
        println!("{}", get_entry_content(&entry));
    }
}

fn setup_db(connection: &sqlite::Connection) {
    connection
        .execute("CREATE TABLE IF NOT EXISTS FEEDS ( TITLE TEXT PRIMARY KEY, URL TEXT UNIQUE);")
        .unwrap();
    connection.execute("CREATE TABLE IF NOT EXISTS FEED_ITEMS ( FEED_TITLE TEXT REFERENCES FEEDS(TITLE), TITLE TEXT, URL TEXT, CONTENT TEXT);").unwrap();
}

fn import_opml_into_db(opml_path: &str, connection: &sqlite::Connection) {
    println!("OPML path: {}", opml_path);

    let mut insert_stmt = connection
        .prepare("INSERT OR REPLACE INTO FEEDS (TITLE, URL) VALUES (?, ?);")
        .unwrap();

    let opml_content = std::fs::read_to_string(opml_path).expect("unable to read opml file!");
    let document = opml::OPML::from_str(&opml_content).unwrap();
    for outline in &document.body.outlines {
        let outline_text = &outline.text; // text could have HTML markup
        if let Some(outline_type) = &outline.r#type {
            if outline_type == "rss" {
                println!(" RSS: {}", outline_text);
                if let Some(outline_url) = &outline.xml_url {
                    insert_stmt.reset().unwrap();
                    insert_stmt.bind(1, outline_text.as_str()).unwrap();
                    insert_stmt.bind(2, outline_url.as_str()).unwrap();
                    insert_stmt.next().unwrap();
                    println!("inserted {}", outline_text);
                } else {
                    println!(" No XML URL: {}", outline_text);
                }
            } else {
                println!(" Not RSS ({}): {}", outline_type, outline_text);
            }
        } else {
            println!(" Unknown type: {}", outline_text);
        }
    }
}

fn import_feed_into_db(title: &str, url: &str, connection: &sqlite::Connection) {
    let mut insert_stmt = connection.prepare("INSERT OR REPLACE INTO FEED_ITEMS (FEED_TITLE, TITLE, URL, CONTENT) VALUES (?, ?, ?, ?);").unwrap();
    let response = reqwest::blocking::get(url);
    if let Err(e) = response {
        println!("  Failed to read '{}' from {}: {}", title, url, e);
        return;
    }
    let feed = feed_rs::parser::parse(response.unwrap());
    if let Err(e) = feed {
        println!("  Failed to parse '{}' from {}: {}", title, url, e);
        return;
    }
    for entry in &feed.unwrap().entries {
        insert_stmt.reset().unwrap();
        insert_stmt.bind(1, title).unwrap();
        insert_stmt
            .bind(2, text_content_or(&entry.title, "(no title)"))
            .unwrap();
        let (entry_content, entry_url) = get_entry_content_and_url(&entry);
        insert_stmt.bind(3, entry_url.as_str()).unwrap();
        insert_stmt.bind(4, entry_content.as_str()).unwrap();
        insert_stmt.next().unwrap();
    }
}

fn refresh_feeds(connection: &sqlite::Connection) {
    let mut select_stmt = connection.prepare("SELECT TITLE, URL FROM FEEDS;").unwrap();
    let mut items = std::vec::Vec::<(String, String)>::new();
    while let sqlite::State::Row = select_stmt.next().unwrap() {
        items.push((
            select_stmt.read::<String>(0).unwrap(),
            select_stmt.read::<String>(1).unwrap(),
        ));
    }

    println!("Found {} feeds to refresh...", items.len());
    for item in items {
        import_feed_into_db(item.0.as_str(), item.1.as_str(), &connection);
    }
}

pub struct Config {
    pub db_path: String,
    pub opml_path: String,
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    // let opml_path = r"C:\Users\mruiz\OneDrive\Documents\2021\feeds.opml";
    // let db_path = r"C:\Users\mruiz\AppData\Local\Temp\plain-rss.db"; // :memory:

    let connection = sqlite::open(config.db_path).unwrap();
    setup_db(&connection);

    if 1 < 2 {
        import_opml_into_db(&config.opml_path, &connection);
        refresh_feeds(&connection);
    } else {
        let opml_content =
            std::fs::read_to_string(config.opml_path).expect("unable to read opml file!");
        let document = opml::OPML::from_str(&opml_content).unwrap();
        for outline in &document.body.outlines {
            let outline_text = &outline.text; // text could have HTML markup
            if let Some(outline_type) = &outline.r#type {
                if outline_type == "rss" {
                    println!(" RSS: {}", outline_text);
                    if let Some(outline_url) = &outline.xml_url {
                        println!("  {}", outline_url);
                        //process_outline(&outline_text, &outline_url)
                    } else {
                        println!(" No XML URL: {}", outline_text);
                    }
                } else {
                    println!(" Not RSS ({}): {}", outline_type, outline_text);
                }
            } else {
                println!(" Unknown type: {}", outline_text);
            }
        }
    }

    Ok(())
}
