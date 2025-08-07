use std::{collections::{HashMap, HashSet}, fs::{read_to_string, File}, io::{BufRead, BufReader, Write}};
use csv::Reader;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Player {
    id: usize,
    player_name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Report {
    #[serde(rename = "startTime")]
    start_time: u64,
    #[serde(rename = "endTime")]
    end_time: u64,
    players: Vec<usize>,
    code: String,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
struct EdgeKey(usize, usize);

impl EdgeKey {
    fn new(a: usize, b: usize) -> Self {
        if a < b {
            EdgeKey(a, b)
        } else {
            EdgeKey(b, a)
        }
    }
}

fn merge_players(players: Vec<Player>) -> (Vec<Player>, HashMap<usize, usize>) {
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut new_players: Vec<Player> = Vec::new();
    let mut old_to_new: HashMap<usize, usize> = HashMap::new();

    for player in players {
        if let Some(&new_id) = name_to_id.get(&player.player_name) {
            old_to_new.insert(player.id, new_id);
        } else {
            let new_id = new_players.len() + 1;
            name_to_id.insert(player.player_name.clone(), new_id);
            new_players.push(Player {
                id: new_id,
                player_name: player.player_name.clone(),
            });
            old_to_new.insert(player.id, new_id);
        }
    }

    (new_players, old_to_new)
}

fn merge_reports(reports: Vec<Report>, id_map: &HashMap<usize, usize>) -> Vec<Report> {
    let mut map: HashMap<String, Report> = HashMap::new();

    for mut report in reports {
        let new_players: HashSet<usize> = report
            .players
            .iter()
            .filter_map(|id| id_map.get(id).copied())
            .collect();

        report.players = new_players.into_iter().collect();

        map.entry(report.code.clone())
            .and_modify(|existing| {
                existing.start_time = existing.start_time.min(report.start_time);
                existing.end_time = existing.end_time.max(report.end_time);
                let combined: HashSet<usize> = existing
                    .players
                    .iter()
                    .chain(report.players.iter())
                    .copied()
                    .collect();
                existing.players = combined.into_iter().collect();
            })
            .or_insert(report);
    }

    map.into_values().collect()
}

fn read_reports_from_file(path: &str) -> Result<Vec<Report>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    println!("Attempting to open: {}", path);
    let reader = BufReader::new(file);
    let mut reports = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let report: Report = serde_json::from_str(&line)?;
        reports.push(report);
    }

    Ok(reports)
}

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let player_paths = vec!["dataset1/player_table.csv", "dataset2/player_table.csv", "dataset3/player_table.csv"];
//     let report_paths = vec!["dataset1/report_details.json", "dataset2/report_details.json", "dataset3/report_details.json"];

//     let mut all_players = Vec::new();
//     for path in player_paths {
//         println!("Attempting to open: {}", path);
//         let file = File::open(path)?;
//         let mut rdr = Reader::from_reader(file);
//         for result in rdr.deserialize() {
//             let player: Player = result?;
//             all_players.push(player);
//         }
//     }

//     let (merged_players, id_map) = merge_players(all_players);
//     println!("Merged all players");

//     let mut all_reports = Vec::new();
//     for path in report_paths {
//         let reports = read_reports_from_file(path)?;
//         all_reports.extend(reports);
//     }

//     let merged_reports = merge_reports(all_reports, &id_map);
//     println!("Merged all reports");

//     let mut wtr = csv::Writer::from_path("merged_players.csv")?;
//     for player in &merged_players {
//         wtr.serialize(player)?;
//     }
//     wtr.flush()?;

//     let mut file = File::create("merged_reports.json")?;
//     for report in &merged_reports {
//         let json = serde_json::to_string(report)?;
//         writeln!(file, "{}", json)?;
//     }

//     println!("✅ Merged data written to merged_players.json and merged_reports.json");

//     Ok(())
// }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let player_file = File::open("merged_players.csv")?;
    let mut rdr = csv::Reader::from_reader(player_file);
    let mut id_to_name = HashMap::new();
    for result in rdr.deserialize::<Player>() {
        let p = result?;
        id_to_name.insert(p.id, p.player_name);
    }
    println!("Successfully parsed player_table");

    let reader = BufReader::new(File::open("merged_reports.json")?);
    let stream = Deserializer::from_reader(reader).into_iter::<Report>();

    let mut reports = Vec::new();
    for report_result in stream {
        let report: Report = report_result?;
        reports.push(report);
    }
    println!("Successfully parsed report_details");

    let mut appearance_count: HashMap<usize, usize> = HashMap::new();
    let mut valid_reports = vec![];

    for report in &reports {
        if report.players.len() < 30 {
            valid_reports.push(report);
            for &pid in &report.players {
                *appearance_count.entry(pid).or_insert(0) += 1;
            }
        }
    }

    println!("Total valid reports: {}", reports.len());

    const MINIMUM_APPEARANCES: usize = 200;
    let frequent_players: HashSet<usize> = appearance_count
        .iter()
        .filter(|&(_, &count)| count >= MINIMUM_APPEARANCES)
        .map(|(&pid, _)| pid)
        .collect();
    println!("Frequent player count: {}", frequent_players.len());

    let mut edge_weights: HashMap<EdgeKey, u64> = HashMap::new();

    for report in &valid_reports {
        let duration = report.end_time.saturating_sub(report.start_time).min(7_200_000);
        // if report.start_time < 1672531200 {continue;}
        let filtered_players: Vec<usize> = report
            .players
            .iter()
            .filter(|&&p| frequent_players.contains(&p))
            .copied()
            .collect();

        for i in 0..filtered_players.len() {
            for j in (i + 1)..filtered_players.len() {
                let key = EdgeKey::new(filtered_players[i], filtered_players[j]);
                *edge_weights.entry(key).or_insert(0) += duration;
            }
        }
    }

    println!("Total Raw Edges: {}", edge_weights.len());
    
    let mut per_player: HashMap<usize, Vec<(EdgeKey, u64)>> = HashMap::new();
    const MIN_DURATION: u64 = 8_640_000_0u64 * 1; // 1 day in milliseconds
    for (key @ EdgeKey(a, b), &weight) in &edge_weights {
        if weight < MIN_DURATION {continue;}
        per_player.entry(*a)
            .or_default()
            .push((key.clone(), weight));
        per_player.entry(*b)
            .or_default()
            .push((key.clone(), weight));
    }

    let mut selected_edges = HashSet::new();
    const TOP_K: usize = 30;
    for edges in per_player.values() {
        let mut sorted = edges.clone();
        sorted.sort_unstable_by_key(|&(_, w)| std::cmp::Reverse(w));
        for (key, _) in sorted.iter().take(TOP_K) {
            selected_edges.insert(key.clone());
        }
    }

    println!("Trimmed Edges: {}", selected_edges.len());

    let mut output = File::create("output.gexf")?;
    println!("Creating gexf...");
    writeln!(output, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(output, r#"<gexf xmlns="http://www.gexf.net/1.2draft" version="1.2">"#)?;
    writeln!(output, r#"  <graph mode="static" defaultedgetype="undirected">"#)?;
    writeln!(output, r#"    <nodes>"#)?;
    for &id in frequent_players.iter() {
        if let Some(name) = id_to_name.get(&id) {
            writeln!(output, r#"      <node id="{}" label="{}"/>"#, id, name)?;
        }
    }
    writeln!(output, r#"    </nodes>"#)?;
    writeln!(output, r#"    <edges>"#)?;
    for (i, (EdgeKey(source, target), weight)) in
        edge_weights
            .iter()
            .filter(|(k, _)| selected_edges.contains(k))
            .enumerate()
    {
        let new_weight = f64::log10(*weight as f64);
        // let new_weight = weight / MIN_DURATION;
        writeln!(
            output,
            r#"      <edge id="{}" source="{}" target="{}" weight="{}"/>"#,
            i, source, target, new_weight
        )?;
    }
    writeln!(output, r#"    </edges>"#)?;
    writeln!(output, r#"  </graph>"#)?;
    writeln!(output, r#"</gexf>"#)?;

    Ok(())
}