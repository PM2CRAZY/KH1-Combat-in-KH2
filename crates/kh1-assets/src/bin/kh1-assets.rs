//! `kh1-assets <collection folder>`: reads both Soras' files and prints what they hold.
//! `kh1-assets pose <collection folder> <KH1 motion>`: KH1 Sora's pose every frame, as JSON.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = if a.get(1).map(String::as_str) == Some("extract") {
        // `extract <folder> <kh1|kh2> <name> <out>`: a file from either game's packages.
        let root = a.get(2).cloned().unwrap_or_default();
        let name = a.get(4).cloned().unwrap_or_default();
        let out = a.get(5).cloned().unwrap_or_default();
        let data = if a.get(3).map(String::as_str) == Some("kh1") {
            kh1_data::Kh1Install::locate(&root).and_then(|k| k.read(kh1_data::Source::Original, &name))
        } else {
            kh1_assets::Kh2Install::locate(&root).and_then(|k| k.read(&name))
        };
        data.and_then(|d| std::fs::write(&out, d).map_err(|e| kh2_formats::Error::Format(e.to_string())))
    } else if a.get(1).map(String::as_str) == Some("build") {
        // `build <folder> <out mset>`: KH2 Sora's motion set with KH1's combo motions.
        let root = a.get(2).cloned().unwrap_or_default();
        let out = a.get(3).cloned().unwrap_or_default();
        kh1_assets::build_mset(&root).and_then(|(b, built)| {
            println!("{built:?}");
            std::fs::write(&out, b).map_err(|e| kh2_formats::Error::Format(e.to_string()))
        })
    } else if a.get(1).map(String::as_str) == Some("pose") {
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
