use invoice_lookup::InvoiceLookupClient;
use std::collections::{HashMap, HashSet};
use std::fs;

#[tokio::main]
async fn main() {
    let csv_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "history.csv".to_string());
    let content = fs::read_to_string(&csv_path).expect("Failed to read CSV");
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    // Header: add new column
    if !lines.is_empty() {
        lines[0] = format!("{},登録状況", lines[0]);
    }

    // Collect unique T-numbers from column index 4
    let mut unique_tnums: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for i in 1..lines.len() {
        let cols = parse_csv_line(&lines[i]);
        if cols.len() > 4 {
            let tnum = cols[4].trim();
            if !tnum.is_empty() && tnum != "なし" {
                let normalized = if tnum.starts_with('T') {
                    tnum.to_string()
                } else if tnum.len() == 13 && tnum.chars().all(|c| c.is_ascii_digit()) {
                    format!("T{}", tnum)
                } else {
                    continue;
                };
                if seen.insert(normalized.clone()) {
                    unique_tnums.push(normalized);
                }
            }
        }
    }

    eprintln!("Found {} unique T-numbers to check", unique_tnums.len());

    // Batch lookup
    let client = InvoiceLookupClient::new();
    let refs: Vec<&str> = unique_tnums.iter().map(|s| s.as_str()).collect();
    let results = client.lookup_batch(&refs).await;

    let mut status_map: HashMap<String, bool> = HashMap::new();
    for (tnum, result) in unique_tnums.iter().zip(results.iter()) {
        match result {
            Ok(info) => {
                let digits = tnum.strip_prefix('T').unwrap_or(tnum);
                status_map.insert(digits.to_string(), info.registered);
                status_map.insert(format!("T{}", digits), info.registered);
                eprintln!(
                    "  {} => {}",
                    tnum,
                    if info.registered {
                        "登録済"
                    } else {
                        "未登録"
                    }
                );
            }
            Err(e) => {
                eprintln!("  {} => ERROR: {}", tnum, e);
            }
        }
    }

    // Update each data row
    for i in 1..lines.len() {
        let cols = parse_csv_line(&lines[i]);
        let status = if cols.len() > 4 {
            let tnum = cols[4].trim();
            if tnum.is_empty() || tnum == "なし" {
                "番号なし"
            } else {
                let normalized = if tnum.starts_with('T') {
                    tnum.to_string()
                } else {
                    format!("T{}", tnum)
                };
                match status_map.get(&normalized) {
                    Some(true) => "登録済",
                    Some(false) => "未登録",
                    None => "確認不可",
                }
            }
        } else {
            "番号なし"
        };
        lines[i] = format!("{},{}", lines[i], status);
    }

    // Write back
    let output = lines.join("\n");
    fs::write(&csv_path, &output).expect("Failed to write CSV");
    eprintln!("Updated {} rows in {}", lines.len() - 1, csv_path);
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == ',' && !in_quotes {
            fields.push(current.clone());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}
