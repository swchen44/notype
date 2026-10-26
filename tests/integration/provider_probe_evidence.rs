//! Public, synthetic contract documents only. No provider/home/account lookup.
use super::probe::evidence::validate_page;
use serde_json::json;

#[cfg(windows)]
#[test]
fn actual_windows_available_space_and_sha256_work_on_owned_fixture() {
    let directory = tempfile::tempdir().unwrap();
    assert!(super::probe::native::available(directory.path()).unwrap() > 0);
    assert_eq!(
        super::probe::native::sha256(b"abc").unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

fn pricing(input: &str, output: &str) -> String {
    format!(
        r#"<p>Old comparison: deepseek-flash $0.3 $1.2</p><table>
    <tr><td colspan="3">MODEL</td><td>deepseek-flash<sup>(1)</sup></td><td>another-model</td></tr>
    <tr><td colspan="3">MODEL VERSION</td><td>DeepSeek-V4.1-Flash-1017</td><td>other-version</td></tr>
    <tr><td rowspan="4">PRICING<sup>(2)</sup></td><td rowspan="2">1M INPUT TOKENS<br>(CACHE MISS)</td><td>OFF-PEAK</td><td>$0.15</td><td>$0.3</td></tr>
    <tr><td>PEAK</td><td>{input}</td><td>$0.3</td></tr>
    <tr><td rowspan="2">1M OUTPUT TOKENS</td><td>OFF-PEAK</td><td>$0.6</td><td>$1.2</td></tr>
    <tr><td>PEAK</td><td>{output}</td><td>$1.2</td></tr></table>"#
    )
}
#[test]
fn pricing_binds_rowspan_peak_rates_to_flash_column_despite_old_price_decoys() {
    assert!(validate_page(0, pricing("$0.30", "$1.20").as_bytes()).is_ok());
    for (input, output) in [("$0.31", "$1.20"), ("$0.30", "$1.21"), ("$0.15", "$0.60")] {
        assert!(validate_page(0, pricing(input, output).as_bytes()).is_err());
    }
    let valid = pricing("$0.3", "$1.2");
    assert!(validate_page(0, format!("{valid}{valid}").as_bytes()).is_err());
    assert!(validate_page(
        0,
        valid
            .replace("DeepSeek-V4.1-Flash-1017", "changed-version")
            .as_bytes()
    )
    .is_err());
}
#[test]
fn free_metadata_requires_one_active_fixed_route_all_text_prices_and_requested_controls() {
    let endpoint = json!({"tag":"google-ai-studio","status":0,"pricing":{"prompt":"0","completion":0.0,"request":"0.00","input_cache_read":"0"},"supported_parameters":["reasoning","max_tokens"]});
    let document = |value| json!({"data":{"endpoints":[value]}}).to_string();
    assert!(validate_page(3, document(endpoint.clone()).as_bytes()).is_ok());
    for (field, value) in [
        ("request", json!("0.0000000000000000001")),
        ("prompt", json!(0.0001)),
        ("completion", serde_json::Value::Null),
    ] {
        let mut changed = endpoint.clone();
        changed["pricing"][field] = value;
        assert!(validate_page(3, document(changed).as_bytes()).is_err());
    }
    for (field, value) in [
        ("status", json!(1)),
        ("tag", json!("another-provider")),
        ("supported_parameters", json!(["max_tokens"])),
    ] {
        let mut changed = endpoint.clone();
        changed[field] = value;
        assert!(validate_page(3, document(changed).as_bytes()).is_err());
    }
    let underflow = document(endpoint).replace("\"prompt\":\"0\"", "\"prompt\":1e-999");
    assert!(validate_page(3, underflow.as_bytes()).is_err());
}
