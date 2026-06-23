use std::{
    fs,
    io::{Read, Write},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn native_host_uses_chrome_length_framing() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let setting_dir = std::env::temp_dir().join(format!("legra-native-host-test-{unique}"));
    fs::create_dir_all(&setting_dir).unwrap();
    fs::write(
        setting_dir.join("app-data.json"),
        r#"{"papers":[],"settings":{"managed_directory":null,"workspace_root":null}}"#,
    )
    .unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_paper_manager_native_host"))
        .env("LEGRA_SETTING_DIR", &setting_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let request = br#"{"action":"list_categories"}"#;
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

    assert_eq!(
        response.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );
    assert!(response
        .get("categories")
        .and_then(|value| value.as_array())
        .is_some_and(|values| values.is_empty()));
    assert!(child.wait().unwrap().success());

    let _ = fs::remove_dir_all(setting_dir);
}
