mod domain;
mod infrastructure;
mod presentation;
mod usecase;

use std::sync::Arc;

use clap::Parser;

use domain::repository::InvoiceRepository;
use infrastructure::nta_repository::NtaWebRepository;
use presentation::commands::{Cli, Commands};
use usecase::check_history::CheckHistoryUseCase;
use usecase::lookup::LookupInvoiceUseCase;

/// Bootstrap: 依存性の注入とアプリケーション初期化
fn bootstrap() -> AppContext {
    let repo: Arc<dyn InvoiceRepository> = Arc::new(NtaWebRepository::new());
    AppContext {
        lookup_usecase: LookupInvoiceUseCase::new(Arc::clone(&repo)),
        check_history_usecase: CheckHistoryUseCase::new(Arc::clone(&repo)),
    }
}

struct AppContext {
    lookup_usecase: LookupInvoiceUseCase,
    check_history_usecase: CheckHistoryUseCase,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let ctx = bootstrap();

    match cli.command {
        Commands::Lookup { t_number, json } => {
            handle_lookup(&ctx, &t_number, json).await;
        }
        Commands::Batch {
            numbers,
            file,
            json,
        } => {
            handle_batch(&ctx, numbers, file, json).await;
        }
        Commands::CheckHistory { input, output } => {
            handle_check_history(&ctx, &input, output.as_deref()).await;
        }
    }
}

async fn handle_lookup(ctx: &AppContext, t_number: &str, json: bool) {
    match ctx.lookup_usecase.execute(t_number).await {
        Ok(info) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&info).unwrap());
            } else {
                let status = if info.registered { "登録済" } else { "未登録" };
                println!("T番号:       {}", info.t_number);
                println!("ステータス:  {}", status);
                if info.registered {
                    println!("名称:       {}", info.name);
                    println!("登録日:     {}", info.registration_date);
                    println!("所在地:     {}", info.address);
                    println!("最終更新:   {}", info.last_updated);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_batch(
    ctx: &AppContext,
    numbers: Vec<String>,
    file: Option<String>,
    json: bool,
) {
    let mut all_numbers = numbers;

    if let Some(path) = file {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        all_numbers.push(trimmed.to_string());
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading file {}: {}", path, e);
                std::process::exit(1);
            }
        }
    }

    if all_numbers.is_empty() {
        eprintln!("No T-numbers specified. Use -n or -f option.");
        std::process::exit(1);
    }

    let results = ctx.lookup_usecase.execute_batch(&all_numbers).await;

    if json {
        let entries: Vec<serde_json::Value> = results
            .iter()
            .map(|r| match r {
                Ok(info) => serde_json::to_value(info).unwrap(),
                Err(e) => serde_json::json!({"error": e}),
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries).unwrap());
    } else {
        for result in &results {
            match result {
                Ok(info) => {
                    let status = if info.registered { "登録済" } else { "未登録" };
                    println!("{} => {} {}", info.t_number, status, info.name);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }
    }
}

async fn handle_check_history(ctx: &AppContext, input: &str, output: Option<&str>) {
    let content = match std::fs::read_to_string(input) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", input, e);
            std::process::exit(1);
        }
    };

    let result = ctx.check_history_usecase.execute(&content).await;
    let summary = result.summary();
    let csv_output = result.to_csv();

    let out_path = output.unwrap_or(input);
    if let Err(e) = std::fs::write(out_path, &csv_output) {
        eprintln!("Error writing {}: {}", out_path, e);
        std::process::exit(1);
    }

    eprintln!("=== 処理結果 ===");
    eprintln!("  合計:     {} 行", summary.total);
    eprintln!("  登録済:   {}", summary.registered);
    eprintln!("  登録前:   {}", summary.before_registration);
    eprintln!("  未登録:   {}", summary.unregistered);
    eprintln!("  番号なし: {}", summary.no_number);
    eprintln!("  エラー:   {}", summary.errors);
    if !summary.unregistered_numbers.is_empty() {
        eprintln!("  未登録番号: {}", summary.unregistered_numbers.join(", "));
    }
    if !summary.before_registration_entries.is_empty() {
        eprintln!(
            "  登録前エントリ: {}",
            summary.before_registration_entries.join(", ")
        );
    }
    eprintln!("出力: {}", out_path);
}
