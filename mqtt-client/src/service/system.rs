use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

pub async fn device_info() -> String {
    let mut sys = System::new();

    sys.refresh_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::nothing().with_ram()),
    );

    // Wait a bit because CPU usage is based on diff.
    tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;

    // CPU
    let cpu_arch = System::cpu_arch();
    let cpu_usage = sys.global_cpu_usage().to_string();
    let cpu_cores = sys.physical_core_count();

    // Memory
    let total_memory = sys.total_memory();
    let used_memory = total_memory - sys.available_memory();

    // OS
    let os_name = System::name();
    let os_kernel = System::kernel_version();
    let os_version = System::os_version();

    // Uptime
    let uptime = System::uptime();

    serde_json::json!({
        "client": {
            "version": VERSION
        },
        "cpu": {
            "arch": cpu_arch,
            "cores": cpu_cores,
            "usage": cpu_usage,
        },
        "memory": {
            "total": total_memory,
            "used": used_memory,
        },
        "os": {
            "name": os_name,
            "kernel": os_kernel,
            "version": os_version,
        },
        "uptime": uptime,
    })
    .to_string()
}

static VERSION: &str = env!("MQTT_CLIENT_VERSION");
