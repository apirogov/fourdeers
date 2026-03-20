fn main() {
    // Get git commit hash
    let commit_hash = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            let hash = String::from_utf8(output.stdout).ok()?;
            Some(hash.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Get current build timestamp
    let build_time = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%d %H:%M:%S UTC"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string());

    // Set cargo env variables
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", commit_hash);
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
}
