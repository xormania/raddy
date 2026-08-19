#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args
        .iter()
        .any(|a| a == "--print-config" || a == "-h" || a == "--help")
    {
        let out = raddy::run_from_process();
        if !out.stdout.is_empty() {
            print!("{}", out.stdout);
        }
        if !out.stderr.is_empty() {
            eprint!("{}", out.stderr);
        }
        std::process::exit(out.exit_code);
    }

    match raddy::load_from_process() {
        Ok(cfg) => {
            if let Err(err) = raddy::run_http(cfg).await {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    }
}
