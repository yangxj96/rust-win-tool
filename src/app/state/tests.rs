use super::*;

#[test]
fn initial_operation_is_idle() {
    assert_eq!(AppState::new().operation_state(), OperationState::Idle);
}

#[test]
fn backend_status_is_typed() {
    assert_eq!(
        ServiceStatus::from_backend("Running"),
        ServiceStatus::Running
    );
    assert_eq!(
        ServiceStatus::from_backend("Stopped"),
        ServiceStatus::Stopped
    );
    assert_eq!(
        ServiceStatus::from_backend("unexpected"),
        ServiceStatus::Unknown
    );
}

#[test]
fn tools_are_selectable_up_to_the_catalog_size() {
    let mut state = AppState::new();

    state.select_tool(5);
    assert_eq!(state.tools_selected(), 0);

    state.select_tool(2);
    assert_eq!(state.tools_selected(), 2);
    assert!(state.open_tool_detail());
    assert!(state.tool_detail_active());
    assert!(state.monitor_active());
}

#[test]
fn dialog_filter_matches_name_and_display_name() {
    let mut state = AppState::new();
    state.begin_add_dialog();
    state.set_add_dialog_services(Ok(vec![
        ServiceInfo {
            name: "AlphaSvc".into(),
            display_name: "Alpha Display".into(),
            status: "Stopped".into(),
            start_type: "Automatic".into(),
            description: String::new(),
        },
        ServiceInfo {
            name: "BetaSvc".into(),
            display_name: "Beta Display".into(),
            status: "Running".into(),
            start_type: "Manual".into(),
            description: String::new(),
        },
    ]));
    state.add_dialog_commit_text("b");
    assert_eq!(state.add_dialog_filtered(), &[1]);
}

fn add_managed(state: &mut AppState, name: &str, display_name: &str) {
    state.managed_services.push(ManagedService {
        name: name.into(),
        display_name: display_name.into(),
        enabled: true,
    });
}

#[test]
fn service_list_filters_by_search_and_status() {
    let mut state = AppState::new();
    add_managed(&mut state, "Redis", "Redis Server");
    add_managed(&mut state, "MySQL", "MySQL");
    state.apply_service_success("Redis", "Running");
    state.apply_service_success("MySQL", "Stopped");

    assert_eq!(state.filtered_service_indices(), vec![0, 1]);

    state.service_search_commit_text("red");
    assert_eq!(state.filtered_service_indices(), vec![0]);

    state.service_search_backspace();
    state.service_search_backspace();
    state.service_search_backspace();
    assert_eq!(state.filtered_service_indices(), vec![0, 1]);

    state.set_service_filter(ServiceFilter::Running);
    assert_eq!(state.filtered_service_indices(), vec![0]);
    state.set_service_filter(ServiceFilter::Stopped);
    assert_eq!(state.filtered_service_indices(), vec![1]);
    state.set_service_filter(ServiceFilter::Pending);
    assert!(state.filtered_service_indices().is_empty());
}

#[test]
fn service_list_sorts_by_name_and_status() {
    let mut state = AppState::new();
    add_managed(&mut state, "Alpha", "Alpha");
    add_managed(&mut state, "Bravo", "Bravo");
    state.apply_service_success("Alpha", "Stopped");
    state.apply_service_success("Bravo", "Running");

    assert_eq!(state.filtered_service_indices(), vec![0, 1]);

    state.cycle_service_sort(); // 名称
    assert_eq!(state.filtered_service_indices(), vec![0, 1]);

    state.cycle_service_sort(); // 状态：运行中的服务优先
    assert_eq!(state.filtered_service_indices(), vec![1, 0]);
}

#[test]
fn operation_history_is_capped_and_cleared() {
    let mut state = AppState::new();
    for index in 0..(HISTORY_LIMIT + 10) {
        state.record_history(HistoryEntry {
            action: HistoryAction::Start,
            service: format!("S{index}"),
            ok: index % 2 == 0,
            message: String::new(),
        });
    }
    assert_eq!(state.history().len(), HISTORY_LIMIT);
    assert!(state
        .history()
        .last()
        .is_some_and(|entry| entry.service == "S109"));

    state.clear_history();
    assert!(state.history().is_empty());
}

#[test]
fn failed_operation_keeps_last_known_status() {
    let mut state = AppState::new();
    add_managed(&mut state, "Svc", "Svc");
    assert_eq!(state.service_status("Svc"), ServiceStatus::Unknown);

    state.apply_service_success("Svc", "Running");
    assert_eq!(state.service_status("Svc"), ServiceStatus::Running);

    state.apply_service_error("Svc", &BackendError::Service("access is denied".into()));
    assert_eq!(state.service_status("Svc"), ServiceStatus::Running);
    assert!(state.service_message("Svc").is_some());
}

#[test]
fn dialog_selection_accepts_a_filtered_index() {
    let mut state = AppState::new();
    state.begin_add_dialog();
    state.set_add_dialog_services(Ok(vec![
        ServiceInfo {
            name: "AlphaSvc".into(),
            display_name: "Alpha Display".into(),
            status: "Stopped".into(),
            start_type: "Automatic".into(),
            description: String::new(),
        },
        ServiceInfo {
            name: "BetaSvc".into(),
            display_name: "Beta Display".into(),
            status: "Running".into(),
            start_type: "Manual".into(),
            description: String::new(),
        },
    ]));

    state.select_add_dialog(1);

    assert_eq!(state.add_dialog_selected(), 1);
}

#[test]
fn script_balance_check_flags_unbalanced_powershell() {
    assert!(super::scripts::balanced(
        "Get-Process | Where-Object { $_.CPU -gt 1 }"
    ));
    assert!(super::scripts::balanced("Write-Output 'a (b'"));
    assert!(!super::scripts::balanced("if ($x) { Write-Host 'hi'"));
    assert!(!super::scripts::balanced("Write-Output \"unterminated"));
}

#[test]
fn custom_script_dialog_validates_and_saves() {
    let mut state = AppState::new();
    // 将持久化路径重定向到临时目录，避免测试触及真实配置文件。
    state.scripts_file = std::env::temp_dir().join("rust-win-tool-test-scripts.json");
    let _ = std::fs::remove_file(&state.scripts_file);

    state.begin_add_script();
    state.confirm_add_script();
    assert!(state.script_dialog_error().is_some());
    assert!(state.custom_scripts().is_empty());

    state.script_dialog_name_commit_text("My Script");
    state.confirm_add_script();
    assert!(state.script_dialog_error().is_some());

    state.set_script_dialog_kind(ScriptKind::PowerShell);
    assert!(state.script_dialog_error().is_none());
    state.script_dialog_content_commit_text("if ($true) {");
    state.confirm_add_script();
    assert!(state.script_dialog_error().is_some());

    state.script_dialog_content_unmark();
    let len = state.script_dialog_content_utf16_len();
    state.script_dialog_content_replace_range(0..len, "Write-Output 'ok'");
    state.confirm_add_script();
    assert!(state.script_dialog_error().is_none());
    assert_eq!(state.custom_scripts().len(), 1);

    let _ = std::fs::remove_file(&state.scripts_file);
}

#[test]
fn custom_script_rejects_duplicate_names() {
    let mut state = AppState::new();
    state.scripts_file = std::env::temp_dir().join("rust-win-tool-test-dup-scripts.json");
    let _ = std::fs::remove_file(&state.scripts_file);
    state.custom_scripts.push(CustomScript {
        name: "Existing".into(),
        kind: ScriptKind::Cmd,
        command: "echo hi".into(),
    });

    state.begin_add_script();
    state.script_dialog_name_commit_text("existing");
    state.script_dialog_content_commit_text("echo hi");
    state.confirm_add_script();

    assert!(state.script_dialog_error().is_some());
    assert_eq!(state.custom_scripts().len(), 1);
    let _ = std::fs::remove_file(&state.scripts_file);
}
