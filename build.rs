fn main() {
    let commit_hash = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            let hash = String::from_utf8(output.stdout).ok()?;
            Some(hash.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let build_time = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();

    println!("cargo:rustc-env=GIT_COMMIT_HASH={commit_hash}");
    println!("cargo:rustc-env=BUILD_TIME={build_time}");
}
