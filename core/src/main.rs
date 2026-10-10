use sysinfo::{Disks, System};

struct SystemSnapshot {
    memory: MemoryInfo,
    system: SystemInfo,
    cpu: CpuInfo,
    disk: Vec<DiskInfo>,
}
struct SystemInfo {
    hostname: Option<String>,
    os_name: Option<String>,
    kernel_version: Option<String>,
    uptime_seconds: u64,
}
struct MemoryInfo {
    total_bytes: u64,
    used_bytes: u64,
}
struct CpuInfo {
    logical_cpu_count: usize,
    usage_percent: f32,
}
struct DiskInfo {
    name: String,
    total_bytes: u64,
    available_bytes: u64,
}
fn collect_memory() -> MemoryInfo {
    let mut system = System::new();
    system.refresh_memory();

    let memory = MemoryInfo {
        total_bytes: system.total_memory(),
        used_bytes: system.used_memory(),
    };
    memory
}
fn collect_system_info() -> SystemInfo {
    let system = SystemInfo {
        hostname: System::host_name(),
        os_name: System::name(),
        kernel_version: System::kernel_version(),
        uptime_seconds: System::uptime(),
    };
    system
}
fn collect_disks() -> Vec<DiskInfo> {
    let disks = Disks::new_with_refreshed_list();
    let mut disk_infos: Vec<DiskInfo> = Vec::new();

    for disk in disks.list() {
        let d = DiskInfo {
            name: disk.name().to_string_lossy().into_owned(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
        };
        disk_infos.push(d);
    }
    disk_infos
}
fn collect_cpu_info() -> CpuInfo {
    let mut system = System::new();
    system.refresh_cpu_all();
    let cpu_count = system.cpus().len();
    if cpu_count == 0 {
        return CpuInfo {
            logical_cpu_count: 0,
            usage_percent: 0.0,
        };
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    system.refresh_cpu_all();
    let mut total_usage = 0.0;
    for cpu in system.cpus() {
        total_usage += cpu.cpu_usage();
    }

    let cpu = CpuInfo {
        logical_cpu_count: cpu_count,
        usage_percent: total_usage / cpu_count as f32,
    };
    cpu
}
fn collect_snapshot() -> SystemSnapshot {
    let snapshot = SystemSnapshot {
        memory: collect_memory(),
        system: collect_system_info(),
        cpu: collect_cpu_info(),
        disk: collect_disks(),
    };
    snapshot
}
fn print_snapshot(snapshot: &SystemSnapshot) {
    let t_gb = snapshot.memory.total_bytes as f64 / 1024_f64.powi(3);
    let u_gb = snapshot.memory.used_bytes as f64 / 1024_f64.powi(3);
    let hostname = &snapshot.system.hostname;
    let os_name = &snapshot.system.os_name;
    let kernel_version = &snapshot.system.kernel_version;
    let uptime = snapshot.system.uptime_seconds;
    let hours = uptime / 3600;
    let minutes = (uptime % 3600) / 60;
    let cpu_count = snapshot.cpu.logical_cpu_count;
    let cpu_usage = snapshot.cpu.usage_percent;
    println!("Total RAM: {:.2} GiB", t_gb);
    println!("Used RAM: {:.2} GiB ", u_gb);
    println!("Host name: {:?}", hostname);
    println!("OS name: {:?}", os_name);
    println!("Kernel Version: {:?}", kernel_version);
    println!("Uptime: {} hrs {} mins", hours, minutes);
    println!("Total number of cpus: {}", cpu_count);
    println!("Percentage of CPU used: {}%", cpu_usage);

    for disk in &snapshot.disk {
        let total_gib = disk.total_bytes as f64 / 1024_f64.powi(3);
        let available_gib = disk.available_bytes as f64 / 1024_f64.powi(3);
        println!("Disk: {}", disk.name);
        println!("Total: {:.2} GiB", total_gib);
        println!("Available: {:.2} GiB", available_gib);
    }
}
fn main() {
    let snapshot = collect_snapshot();
    print_snapshot(&snapshot);
}
