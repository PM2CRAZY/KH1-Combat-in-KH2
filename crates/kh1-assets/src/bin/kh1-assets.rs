//! `kh1-assets <collection folder>`: reads both Soras' files and prints what they hold (a check of
//! the build step's readers on the player's own install).
fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    match kh1_assets::scan(&root) {
        Ok(r) => println!("{r:#?}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
