//! `kh1-assets <collection folder>`: reads both Soras' files and prints what they hold.
//! `kh1-assets pose <collection folder> <KH1 motion>`: KH1 Sora's pose every frame, as JSON.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = if a.get(1).map(String::as_str) == Some("pose") {
        let root = a.get(2).cloned().unwrap_or_else(|| ".".into());
        let idx = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
        kh1_assets::kh1_pose_json(&root, idx).map(|s| println!("{s}"))
    } else {
        kh1_assets::scan(a.get(1).cloned().unwrap_or_else(|| ".".into())).map(|r| println!("{r:#?}"))
    };
    if let Err(e) = r {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
