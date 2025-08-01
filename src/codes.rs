use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const CODE_SETTINGS: &str = "code_settings.txt";
const CODES_FILE: &str = "sorted_report_codes.txt";
const REPORTS_OUT: &str = "report_details.json";
const PLAYER_TABLE: &str = "player_table.csv";
const CHECKPOINT_INTERVAL: usize = 25; 

fn read_last_index<P: AsRef<Path>>(path: P) -> io::Result<usize> {
    if let Ok(file) = File::open(path) {
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if let Ok(idx) = line.trim().parse() {
            return Ok(idx);
        }
    }
    Ok(0)
}

fn write_last_index<P: AsRef<Path>>(path: P, idx: usize) -> io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "{}", idx)
}

fn append_line<P: AsRef<Path>>(path: P, line: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", line)
}

fn dump_player_map(
    map: &HashMap<String, usize>,
) -> io::Result<()> {
    let mut table = File::create(PLAYER_TABLE)?;
    writeln!(table, "id,player_name")?;
    let mut inv: Vec<(usize, &String)> =
        map.iter().map(|(n, &i)| (i, n)).collect();
    inv.sort_unstable_by_key(|(i, _)| *i);
    for (id, name) in inv {
        writeln!(table, "{},{}", id, name)?;
    }
    Ok(())
}

fn load_player_map() -> io::Result<(HashMap<String,usize>, usize)> {
    let mut map = HashMap::new();
    let mut max_id = 0;

    if Path::new(PLAYER_TABLE).exists() {
        let file = File::open(PLAYER_TABLE)?;
        let mut rdr = BufReader::new(file).lines();

        if let Some(Ok(header)) = rdr.next() {
            if header.trim() != "id,player_name" {
                let parts: Vec<_> = header.trim().splitn(2, ',').collect();
                if parts.len() == 2 {
                    if let Ok(id) = parts[0].parse() {
                        map.insert(parts[1].to_string(), id);
                        max_id = max_id.max(id);
                    }
                }
            }
        }

        for line in rdr {
            let line = line?;
            let parts: Vec<_> = line.trim().splitn(2, ',').collect();
            if parts.len() != 2 { continue; }
            if let Ok(id) = parts[0].parse() {
                map.insert(parts[1].to_string(), id);
                max_id = max_id.max(id);
            }
        }
    }

    Ok((map, max_id + 1))
}

fn fetch_report_data(
    client: &Client,
    token: &str,
    report_code: &str,
) -> Result<Option<GraphQLResponse>, Box<dyn std::error::Error>> {
    let url = "https://www.esologs.com/api/v2/client";
    let query = json!({
        "query": format!(
            r#"{{ reportData {{ report(code: "{}") {{ endTime startTime masterData(translate: false) {{ actors(type: "Player") {{ displayName }} }} }} }} }}"#,
            report_code
        )
    });

    let mut attempts = 0;
    loop {
        let res = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", token))
            .json(&query)
            .send()?;

        if res.status().is_success() {
            let text = res.text()?;
            let resp: GraphQLResponse = serde_json::from_str(&text)?;
            return Ok(Some(resp));
        }

        attempts += 1;
        if attempts > 5 {
            return Err(format!("Failed after 5 attempts: {}", res.status()).into());
        }

        eprintln!(
            "Report {} returned {}, backing off… (attempt #{})",
            report_code,
            res.status(),
            attempts
        );
        thread::sleep(Duration::from_secs(10));
    }
}

fn process_codes() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("ESOLOGS_API_TOKEN")?;
    let client = Client::new();

    let file = File::open(CODES_FILE)?;
    let reader = BufReader::new(file);
    let codes: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    let (map, mut next_player_id) = load_player_map()?;
    let player_map = Arc::new(Mutex::new(map));
    let mut since_last_checkpoint = 0usize;

    {
        let pm = Arc::clone(&player_map);
        ctrlc::set_handler(move || {
            eprintln!("Caught Ctrl+C! Dumping player map…");
            if let Ok(map) = pm.lock() {
                if let Err(e) = dump_player_map(&*map) {
                    eprintln!("  Failed to dump: {}", e);
                }
            }
            std::process::exit(1);
        }).expect("Error setting Ctrl-C handler");
    }

    let last_idx = read_last_index(CODE_SETTINGS)?;
        println!("Resuming from index {} …", last_idx);

    for (idx, code) in codes.iter().enumerate().skip(last_idx) {
        println!("Fetching report {}", code);
        match fetch_report_data(&client, &token, code) {
            Ok(Some(report)) => {
                if let Some(report_for_actors) = report.data.report_data.report {
                    if let Some(players) = report_for_actors.master_data.actors {
                        let mut id_list = Vec::with_capacity(players.len());
                        for actor in players {
                            let name = actor.display_name;
                            if name == "nil" { continue; }
                            let mut map_lock = player_map.lock().unwrap();
                            let id = *map_lock.entry(name.clone())
                                .or_insert_with(|| {
                                    let id = next_player_id;
                                    next_player_id += 1;
                                    id
                                });
                            drop(map_lock);
                            id_list.push(id);
                        }
                        let obj = json!({
                            "code": code,
                            "endTime": report_for_actors.end_time,
                            "startTime": report_for_actors.start_time,
                            "players": id_list
                        });
                        println!("Adding {}", obj);

                        append_line(REPORTS_OUT, &obj.to_string())?;
                    }
                }
            }
            Ok(None) => {
                eprintln!("No report data for code {}", code);
            }
            Err(err) => {
                eprintln!("Error fetching {}: {}", code, err);
                continue;
            }
        }
        write_last_index(CODE_SETTINGS, idx + 1)?;
        since_last_checkpoint += 1;

        if since_last_checkpoint >= CHECKPOINT_INTERVAL {
            println!("Periodic checkpoint: dumping player map");
            dump_player_map(&*player_map.lock().unwrap())?;

            since_last_checkpoint = 0;
        }
    }

    println!("Completed processing all codes.");
    dump_player_map(&*player_map.lock().unwrap())?;
    Ok(())
}

pub fn process_codes_main_function() -> Result<(), Box<dyn std::error::Error>> {
    process_codes()?;
    Ok(())
}


#[derive(Debug, Serialize, Deserialize)]
pub struct GraphQLResponse {
    pub data: ReportDataWrapper2,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportDataWrapper2 {
    #[serde(rename = "reportData")]
    pub report_data: ReportWrapper,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportWrapper {
    pub report: Option<ReportForActors>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportForActors {
    #[serde(rename = "endTime")]
    pub end_time: u64,
    #[serde(rename = "startTime")]
    pub start_time: u64,
    #[serde(rename = "masterData")]
    pub master_data: MasterData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MasterData {
    pub actors: Option<Vec<Actor>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Actor {
    #[serde(rename = "displayName")]
    pub display_name: String,
}
