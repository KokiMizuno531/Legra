use paper_manager_lib::agent_bridge::{
    agent_import_dirs, claim_pending_import, enqueue_import, import_status,
    recover_stale_processing_imports, AgentImportResult, AgentJobState, DownloadPolicy,
    EnqueueImportInput,
};
use std::{
    fs::{self, FileTimes, OpenOptions},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "legra-agent-recovery-{}-{unique}-{counter}",
        std::process::id()
    ))
}

fn import_input(identifier: &str, key: &str) -> EnqueueImportInput {
    EnqueueImportInput {
        identifier: identifier.to_string(),
        category: Some("Physics/Recovery".to_string()),
        tags: vec!["recovery-test".to_string()],
        download_policy: DownloadPolicy::MetadataOnly,
        idempotency_key: Some(key.to_string()),
        origin: "queue-recovery-test".to_string(),
    }
}

fn make_stale(path: &std::path::Path) {
    OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(FileTimes::new().set_modified(UNIX_EPOCH))
        .unwrap();
}

#[test]
fn queue_recovers_interrupted_publication_and_processing() {
    let setting_dir = unique_temp_dir();
    fs::create_dir_all(&setting_dir).unwrap();
    std::env::set_var("LEGRA_SETTING_DIR", &setting_dir);

    let queued = enqueue_import(import_input("10.1234/recovery", "stable-key")).unwrap();
    let dirs = agent_import_dirs().unwrap();
    let pending = dirs.pending.join(format!("{}.json", queued.request.job_id));

    // Simulate a producer crash after the idempotency record was published but
    // before its pending file survived. An identical retry repairs the queue.
    fs::remove_file(&pending).unwrap();
    let retried = enqueue_import(import_input("10.1234/recovery", "stable-key")).unwrap();
    assert!(retried.already_exists);
    assert_eq!(retried.state, AgentJobState::Pending);
    assert!(pending.is_file());

    let conflict = enqueue_import(import_input("10.1234/different", "stable-key"));
    assert!(conflict.unwrap_err().contains("different import arguments"));

    let processing = claim_pending_import(&queued.request.job_id)
        .unwrap()
        .unwrap();
    let result = AgentImportResult::completed(&queued.request.job_id);
    fs::write(
        dirs.done
            .join(format!("{}.result.json", queued.request.job_id)),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    make_stale(&processing);
    assert_eq!(
        recover_stale_processing_imports(Duration::from_secs(1)).unwrap(),
        1
    );
    assert_eq!(
        import_status(&queued.request.job_id).unwrap().state,
        AgentJobState::Done
    );

    let requeue = enqueue_import(import_input("2401.12345", "requeue-key")).unwrap();
    let processing = claim_pending_import(&requeue.request.job_id)
        .unwrap()
        .unwrap();
    make_stale(&processing);
    assert_eq!(
        recover_stale_processing_imports(Duration::from_secs(1)).unwrap(),
        1
    );
    assert_eq!(
        import_status(&requeue.request.job_id).unwrap().state,
        AgentJobState::Pending
    );

    std::env::remove_var("LEGRA_SETTING_DIR");
    fs::remove_dir_all(setting_dir).unwrap();
}
