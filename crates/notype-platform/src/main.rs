fn main() {
    println!(
        "{}",
        serde_json::to_string(&notype_platform::host_report()).unwrap()
    );
}
