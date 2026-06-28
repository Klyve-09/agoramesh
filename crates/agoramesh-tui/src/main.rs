//! Binary entry point for the `AgoraMesh` TUI.

use std::path::PathBuf;

use clap::Parser;
use color_eyre::Result;

use agoramesh_tui::terminal::run;

#[derive(Debug, Parser)]
#[command(name = "agoramesh-tui")]
#[command(about = "AgoraMesh 최소 터미널 UI")]
struct Args {
    /// 키, 저장소, 피어, TUI 상태를 저장할 데이터 디렉터리.
    #[arg(long, env = "AGORAMESH_DATA_DIR")]
    data_dir: Option<PathBuf>,

    /// 개발용 평문 키 모드로 실행합니다. 실제 신원에는 사용하지 마세요.
    #[arg(long, env = "AGORAMESH_DEV_INSECURE_PLAINTEXT_KEY")]
    dev_insecure_plaintext_key: bool,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    run(args.data_dir, args.dev_insecure_plaintext_key)?;
    Ok(())
}
