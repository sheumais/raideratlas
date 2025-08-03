use std::{collections::{HashMap, HashSet}, fs::File, io::{BufReader, Write}};
use serde::Deserialize;
use serde_json::Deserializer;

#[derive(Debug, Deserialize)]
struct Player {
    id: usize,
    player_name: String,
}

#[derive(Debug, Deserialize)]
struct Report {
    #[serde(rename="startTime")]
    start_time: u64,
    #[serde(rename="endTime")]
    end_time: u64,
    players: Vec<usize>,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let player_file = File::open("player_table.csv")?;
    let mut rdr = csv::Reader::from_reader(player_file);
    let mut id_to_name = HashMap::new();
    for result in rdr.deserialize::<Player>() {
        let p = result?;
        id_to_name.insert(p.id, p.player_name);
    }
    println!("Successfully parsed player_table");

    let reader = BufReader::new(File::open("report_details.json")?);
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

    const MINIMUM_APPEARANCES: usize = 50;
    let frequent_players: HashSet<usize> = appearance_count
        .iter()
        .filter(|&(_, &count)| count >= MINIMUM_APPEARANCES)
        .map(|(&pid, _)| pid)
        .collect();
    println!("Frequent player count: {}", frequent_players.len());

    let mut edge_weights: HashMap<EdgeKey, u64> = HashMap::new();

    for report in &valid_reports {
        let duration = report.end_time.saturating_sub(report.start_time);
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

    let mut per_player: HashMap<usize, Vec<(EdgeKey, u64)>> = HashMap::new();
    for (key @ EdgeKey(a, b), &weight) in &edge_weights {
        per_player.entry(*a)
            .or_default()
            .push((key.clone(), weight));
        per_player.entry(*b)
            .or_default()
            .push((key.clone(), weight));
    }

    let mut selected_edges = HashSet::new();
    const TOP_K: usize = 15;
    for edges in per_player.values() {
        let mut sorted = edges.clone();
        sorted.sort_unstable_by_key(|&(_, w)| std::cmp::Reverse(w));
        for (key, _) in sorted.iter().take(TOP_K) {
            selected_edges.insert(key.clone());
        }
    }

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