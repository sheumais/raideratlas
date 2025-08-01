// use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
// use serde_json::json;
// use std::env;
// use std::fs::{File, OpenOptions};
// use std::io::{self, BufRead, BufReader, Write};
// use std::path::Path;
// use std::thread;
// use std::time::Duration;
mod codes;

// https://www.esologs.com/v2-api-docs/eso/report.doc.html

// fn read_last_user_id<P: AsRef<Path>>(path: P) -> io::Result<usize> {
//     if let Ok(file) = File::open(path) {
//         let mut reader = BufReader::new(file);
//         let mut line = String::new();
//         reader.read_line(&mut line)?;
//         if let Ok(id) = line.trim().parse() {
//             return Ok(id);
//         }
//     }
//     Ok(1)
// }

// fn write_last_user_id<P: AsRef<Path>>(path: P, user_id: usize) -> io::Result<()> {
//     let mut file = File::create(path)?;
//     writeln!(file, "{}", user_id)
// }

// fn append_codes<P: AsRef<Path>>(path: P, codes: Report) -> io::Result<()> {
//     let mut file = OpenOptions::new()
//         .create(true)
//         .append(true)
//         .open(path)?;
//     writeln!(file, "{}", codes.code)?;
//     Ok(())
// }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let token = env::var("ESOLOGS_API_TOKEN")
    //     .expect("ESOLOGS_API_TOKEN must be set in your environment");

    // let settings_path = "settings.txt";
    // let guild_settings_path = "guild_settings.txt";
    // let start_user_id = read_last_user_id(settings_path)?;

    // let client = Client::new();
    // let url = "https://www.esologs.com/api/v2/client";

    // for user_id in start_user_id..46207 {
    //     let mut page_number = 1;
    //     println!("Starting user_id {}", user_id);
    //     let mut error_count = 0;
    //     loop {
    //         if error_count > 100 {
    //             panic!("error count reached too high a value");
    //         }

    //         let graphql_query = json!({
    //             "query": format!(r#"{{ reportData {{ reports(userID: {}, page: {}) {{ data {{ code }} has_more_pages }} }} }}"#, user_id, page_number),
    //         });

    //         let res = client
    //             .post(url)
    //             .header("Content-Type", "application/json")
    //             .header("Authorization", format!("Bearer {}", token))
    //             .json(&graphql_query)
    //             .send()?;

    //         let status = res.status();
    //         let response_text = res.text()?;
    //         println!("Response: {}", response_text);
    //         if response_text.contains(r#"No user exists"#) {
    //             println!("No user exists");
    //             break;
    //         }

    //         if !status.is_success() {
    //             eprintln!("Request failed (status {}). Retrying soon...", status);
    //             error_count += 1;
    //             thread::sleep(Duration::from_secs(30));
    //             continue;
    //         }

    //         let body: Root = serde_json::from_str(&response_text)
    //             .map_err(|e| format!("Invalid JSON: {}", e))?;
    //         println!("Parsed response for user {} page {}", user_id, page_number);

    //         let reports = match body.data.report_data.reports {
    //             Some(r) => r,
    //             None => {
    //                 eprintln!("Request failed. Possibly the account doesn't exist. Trying again soon...");
    //                 error_count += 1;
    //                 thread::sleep(Duration::from_secs(30));
    //                 continue;
    //             }
    //         };

    //         for codes in reports.data {
    //             append_codes("codes.txt", codes)?;
    //         }

    //         if !reports.has_more_pages {
    //             break;
    //         }
    //         page_number += 1;
    //     }
    //     write_last_user_id(settings_path, user_id)?;
    //     // thread::sleep(Duration::from_millis(200));
    // }

    // let start_guild_id = read_last_user_id(guild_settings_path)?;
    // for guild_id in start_guild_id..5524 {
    //     let mut page_number = 1;
    //     println!("Starting guild_id {}", guild_id);
    //     let mut error_count = 0;
    //     loop {
    //         if error_count > 100 {
    //             panic!("error count reached too high a value");
    //         }

    //         let graphql_query = json!({
    //             "query": format!(r#"{{ reportData {{ reports(guildID: {}, page: {}) {{ data {{ code }} has_more_pages }} }} }}"#, guild_id, page_number),
    //         });

    //         let res = client
    //             .post(url)
    //             .header("Content-Type", "application/json")
    //             .header("Authorization", format!("Bearer {}", token))
    //             .json(&graphql_query)
    //             .send()?;

    //         let status = res.status();
    //         let response_text = res.text()?;
    //         println!("Response: {}", response_text);
    //         if response_text.contains(r#"No guild exists"#) {
    //             println!("No guild exists with that id");
    //             break;
    //         }

    //         if !status.is_success() {
    //             eprintln!("Request failed (status {}). Retrying soon...", status);
    //             error_count += 1;
    //             thread::sleep(Duration::from_secs(30));
    //             continue;
    //         }

    //         let body: Root = serde_json::from_str(&response_text)
    //             .map_err(|e| format!("Invalid JSON: {}", e))?;
    //         println!("Parsed response for guild {} page {}", guild_id, page_number);

    //         let reports = match body.data.report_data.reports {
    //             Some(r) => r,
    //             None => {
    //                 eprintln!("Request failed. Possibly the account doesn't exist. Trying again soon...");
    //                 error_count += 1;
    //                 thread::sleep(Duration::from_secs(30));
    //                 continue;
    //             }
    //         };

    //         for codes in reports.data {
    //             append_codes("codes.txt", codes)?;
    //         }

    //         if !reports.has_more_pages {
    //             break;
    //         }
    //         page_number += 1;
    //     }
    //     write_last_user_id(guild_settings_path, guild_id)?;
    //     // thread::sleep(Duration::from_millis(200));
    // }
    codes::process_codes_main_function()?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Root {
    pub data: ReportDataWrapper,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportDataWrapper {
    #[serde(rename = "reportData")]
    pub report_data: ReportsWrapper,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportsWrapper {
    pub reports: Option<ReportsData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportsData {
    pub data: Vec<Report>,
    pub has_more_pages: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub code: String,
}

// {
//   reportData {
//     report(code: "pb9zkYmVnha4tJQR") {
// 		 endTime,
// 		 startTime,
//       masterData(translate: false) {
//         actors(type: "Player") {
//           displayName
//         }
//       }
//     }
//   }
// }


#[derive(Debug, Deserialize)]
pub struct GraphQLResponse {
    pub data: ReportDataContainer,
}

#[derive(Debug, Deserialize)]
pub struct ReportDataContainer {
    #[serde(rename = "reportData")]
    pub report_data: ReportData,
}

#[derive(Debug, Deserialize)]
pub struct ReportData {
    pub report: Option<ReportForActors>,
}

#[derive(Debug, Deserialize)]
pub struct ReportForActors {
    #[serde(rename = "endTime")]
    pub end_time: u64,
    #[serde(rename = "startTime")]
    pub start_time: u64,
    #[serde(rename = "masterData")]
    pub master_data: MasterData,
}

#[derive(Debug, Deserialize)]
pub struct MasterData {
    pub actors: Option<Vec<Actor>>,
}

#[derive(Debug, Deserialize)]
pub struct Actor {
    #[serde(rename = "displayName")]
    pub display_name: String,
}