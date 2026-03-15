use clap::{Parser, Subcommand};

/// インボイス登録番号検索 CLI
#[derive(Parser)]
#[command(name = "invoice-cli", version, about = "NTA Invoice Registration Lookup CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// T番号でインボイス登録状況を検索する
    Lookup {
        /// T番号 (例: T8013201004026)
        t_number: String,

        /// JSON形式で出力
        #[arg(long, short)]
        json: bool,
    },

    /// 複数のT番号を一括検索する
    Batch {
        /// T番号のリスト (カンマ区切り)
        #[arg(short, long, value_delimiter = ',')]
        numbers: Vec<String>,

        /// T番号リストファイル (1行1番号)
        #[arg(short, long)]
        file: Option<String>,

        /// JSON形式で出力
        #[arg(long, short)]
        json: bool,
    },

    /// CSVファイルのインボイス登録状況を一括チェックする
    CheckHistory {
        /// 入力CSVファイルパス
        #[arg(short, long, default_value = "history.csv")]
        input: String,

        /// 出力CSVファイルパス (省略時は入力ファイルを上書き)
        #[arg(short, long)]
        output: Option<String>,
    },
}
