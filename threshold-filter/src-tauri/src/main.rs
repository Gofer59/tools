#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--daemon") {
        threshold_filter_lib::overlay::run_overlay();
    } else {
        threshold_filter_lib::run();
    }
}
