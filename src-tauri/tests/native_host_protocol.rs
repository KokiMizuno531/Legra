use std::{
    fs,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("legra-{label}-{unique}"))
}

fn send_native_request(setting_dir: &Path, request: &[u8]) -> serde_json::Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_paper_manager_native_host"))
        .env("LEGRA_SETTING_DIR", setting_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(&(request.len() as u32).to_ne_bytes())
        .unwrap();
    stdin.write_all(request).unwrap();
    drop(stdin);

    let mut stdout = child.stdout.take().unwrap();
    let mut length = [0_u8; 4];
    stdout.read_exact(&mut length).unwrap();
    let mut response = vec![0_u8; u32::from_ne_bytes(length) as usize];
    stdout.read_exact(&mut response).unwrap();
    let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
    assert!(child.wait().unwrap().success());
    response
}

#[test]
fn native_host_uses_chrome_length_framing() {
    let setting_dir = unique_temp_dir("native-host-test");
    fs::create_dir_all(&setting_dir).unwrap();
    fs::write(
        setting_dir.join("app-data.json"),
        r#"{"papers":[],"settings":{"managed_directory":null,"workspace_root":null}}"#,
    )
    .unwrap();

    let request = br#"{"action":"list_categories"}"#;
    let response = send_native_request(&setting_dir, request);

    assert_eq!(
        response.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );
    assert!(response
        .get("categories")
        .and_then(|value| value.as_array())
        .is_some_and(|values| values.is_empty()));
    assert_eq!(
        response
            .get("host_version")
            .and_then(|value| value.as_str()),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        response
            .get("category_count")
            .and_then(|value| value.as_u64()),
        Some(0)
    );
    assert_eq!(
        response
            .get("category_source")
            .and_then(|value| value.as_str()),
        Some("local_data")
    );
    assert_eq!(
        response
            .get("managed_directory")
            .and_then(|value| value.as_str()),
        None
    );

    let _ = fs::remove_dir_all(setting_dir);
}

#[test]
fn native_host_lists_managed_library_directory_categories() {
    let setting_dir = unique_temp_dir("native-host-setting");
    let managed_dir = unique_temp_dir("managed-library");
    fs::create_dir_all(managed_dir.join("AI").join("Vision")).unwrap();
    fs::create_dir_all(managed_dir.join("Systems")).unwrap();
    fs::create_dir_all(managed_dir.join(".legra")).unwrap();
    fs::create_dir_all(&setting_dir).unwrap();

    let managed_path = managed_dir.to_string_lossy();
    fs::write(
        setting_dir.join("app-data.json"),
        serde_json::json!({
            "papers": [],
            "settings": {
                "managed_directory": managed_path,
                "workspace_root": null
            }
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        managed_dir.join(".legra").join("library.json"),
        serde_json::json!({
            "schema_version": 1,
            "revision": 3,
            "data": {
                "papers": [
                    { "folder_category": "Theory/Graphs" }
                ],
                "settings": {
                    "managed_directory": null,
                    "workspace_root": null
                }
            }
        })
        .to_string(),
    )
    .unwrap();

    let response = send_native_request(&setting_dir, br#"{"action":"list_categories"}"#);
    assert_eq!(
        response.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );
    let categories = response
        .get("categories")
        .and_then(|value| value.as_array())
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(
        categories,
        vec!["AI", "AI/Vision", "Systems", "Theory", "Theory/Graphs"]
    );
    assert_eq!(
        response
            .get("category_count")
            .and_then(|value| value.as_u64()),
        Some(5)
    );
    assert_eq!(
        response
            .get("category_source")
            .and_then(|value| value.as_str()),
        Some("managed_library")
    );
    assert_eq!(
        response
            .get("managed_directory")
            .and_then(|value| value.as_str()),
        Some(managed_path.as_ref())
    );

    let _ = fs::remove_dir_all(setting_dir);
    let _ = fs::remove_dir_all(managed_dir);
}
