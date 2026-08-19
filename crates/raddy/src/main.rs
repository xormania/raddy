fn main() {
    let out = raddy::run_from_process();
    if !out.stdout.is_empty() {
        print!("{}", out.stdout);
    }
    if !out.stderr.is_empty() {
        eprint!("{}", out.stderr);
    }
    std::process::exit(out.exit_code);
}
