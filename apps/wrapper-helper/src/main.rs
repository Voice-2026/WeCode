fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match wecode_runtime_live::wrapper_helper::handle_args(&args) {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("wecode-wrapper-helper: missing --wecode-wrapper-helper command");
            std::process::exit(64);
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(64);
        }
    }
}
