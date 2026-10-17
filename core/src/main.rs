// Import system information tools from the sysinfo crate.
use sysinfo::{Disks, Pid, ProcessesToUpdate, System};

// Import Instant to measure the total monitoring duration.
use std::time::Instant;

// Stores the final statistics collected during monitoring.
struct ResourceSummary {
    samples_collected: usize,
    average_cpu: Option<f32>,
    min_cpu: Option<f32>,
    max_cpu: Option<f32>,
    average_memory_mib: Option<f64>,
    min_memory_mib: Option<f64>,
    max_memory_mib: Option<f64>,
    average_memory_percentage: Option<f64>,
}

// Stores the thresholds and conditions for triggering resource alerts.
struct AlertConfig {
    max_cpu_per: f32,
    max_ram_per: f32,
    consecutive_alert: u32,
}

// Stores information about a resource alert after it is triggered.
struct ResourceAlert {
    trigger_source: ResourceType,
    usage_per: f32,
    exceeded_threshold: f32,
}

// Defines the resources that can trigger an alert.
enum ResourceType {
    CPU,
    RAM,
}

// Combines information about the system into one snapshot.
struct SystemSnapshot {
    memory: MemoryInfo,
    system: SystemInfo,
    cpu: CpuInfo,
    disk: Vec<DiskInfo>,
    processes: Vec<ProcessInfo>,
}

// Stores general information about the operating system and machine.
struct SystemInfo {
    hostname: Option<String>,
    os_name: Option<String>,
    kernel_version: Option<String>,
    uptime_seconds: u64,
}

// Stores total and used system memory in bytes.
struct MemoryInfo {
    total_bytes: u64,
    used_bytes: u64,
}

// Stores CPU information and overall CPU usage.
struct CpuInfo {
    logical_cpu_count: usize,
    usage_percent: f32,
}

// Stores information about an individual disk.
struct DiskInfo {
    name: String,
    total_bytes: u64,
    available_bytes: u64,
}

// Stores information about an individual running process.
// Clone allows process information to be copied when sorting.
#[derive(Clone)]
struct ProcessInfo {
    pid: Pid,
    name: String,
    cpu_usage: f32,
    memory_bytes: u64,
}

// Collects information about running processes.
fn collect_processes() -> Vec<ProcessInfo> {
    let mut system = System::new();

    // Refresh the process list to collect current process information.
    system.refresh_processes(ProcessesToUpdate::All, true);

    // Wait before refreshing again so CPU usage can be measured.
    std::thread::sleep(std::time::Duration::from_millis(500));
    system.refresh_processes(ProcessesToUpdate::All, true);

    // Create a collection to store the process information.
    let mut process_infos: Vec<ProcessInfo> = Vec::new();

    // Convert each system process into our own ProcessInfo struct.
    for (pid, process) in system.processes() {
        process_infos.push(ProcessInfo {
            pid: *pid,
            name: process.name().to_string_lossy().into_owned(),
            cpu_usage: process.cpu_usage(),
            memory_bytes: process.memory(),
        });
    }

    // Sort processes by CPU usage, highest first.
    process_infos.sort_by(|a, b| b.cpu_usage.total_cmp(&a.cpu_usage));

    process_infos
}

// Returns a new process list sorted by memory usage, highest first.
fn sort_by_memory(processes: &[ProcessInfo]) -> Vec<ProcessInfo> {
    // Clone the original list so it isn't modified.
    let mut sorted = processes.to_vec();

    sorted.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));

    sorted
}

// Prints a summary of the collected process information.
fn summarize_processes(processes: &[ProcessInfo]) {
    // Count the total number of processes.
    let total = processes.len();
    println!("Total processes: {}", total);

    let mut active = 0;
    let mut total_memory: u64 = 0;

    // Count processes using CPU and add up their reported memory.
    for process in processes {
        if process.cpu_usage > 0.0 {
            active += 1;
        }

        total_memory += process.memory_bytes;
    }

    println!("Processes using CPU: {}", active);

    // Convert total process memory from bytes to MiB.
    let total_memory_mib = total_memory as f64 / 1024_f64.powi(2);

    println!("Total process memory: {:.1} MiB", total_memory_mib);
}

// Prints up to the top 20 processes in the supplied order.
fn print_processes(processes: &[ProcessInfo]) {
    println!("\n========== PROCESS ENGINE ==========");

    // Print the table headings.
    println!("{:<8} {:<8} {:<12} {}", "PID", "CPU %", "MEMORY", "NAME");
    println!("{}", "-".repeat(60));

    // Display at most 20 processes.
    for process in processes.iter().take(20) {
        // Convert process memory from bytes to MiB.
        let memory_mib = process.memory_bytes as f64 / 1024_f64.powi(2);

        println!(
            "{:<8} {:<8.1} {:<12.1} {}",
            process.pid, process.cpu_usage, memory_mib, process.name
        );
    }
}

// Collects the current system memory information.
fn collect_memory() -> MemoryInfo {
    let mut system = System::new();

    // Refresh memory statistics before reading them.
    system.refresh_memory();

    let memory = MemoryInfo {
        total_bytes: system.total_memory(),
        used_bytes: system.used_memory(),
    };

    memory
}

// Collects general information about the current system.
fn collect_system_info() -> SystemInfo {
    let system = SystemInfo {
        hostname: System::host_name(),
        os_name: System::name(),
        kernel_version: System::kernel_version(),
        uptime_seconds: System::uptime(),
    };

    system
}

// Collects information about the available disks.
fn collect_disks() -> Vec<DiskInfo> {
    let disks = Disks::new_with_refreshed_list();

    // Store the information for each discovered disk.
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

// Collects CPU information and calculates average CPU usage.
fn collect_cpu_info() -> CpuInfo {
    let mut system = System::new();

    // Refresh CPU information before measuring usage.
    system.refresh_cpu_all();

    let cpu_count = system.cpus().len();

    // Handle systems where no logical CPUs are reported.
    if cpu_count == 0 {
        return CpuInfo {
            logical_cpu_count: 0,
            usage_percent: 0.0,
        };
    }

    // Wait between refreshes so sysinfo can measure CPU activity.
    std::thread::sleep(std::time::Duration::from_millis(500));
    system.refresh_cpu_all();

    // Add up the usage percentages for all logical CPUs.
    let mut total_usage = 0.0;

    for cpu in system.cpus() {
        total_usage += cpu.cpu_usage();
    }

    // Calculate average usage across the logical CPUs.
    let cpu = CpuInfo {
        logical_cpu_count: cpu_count,
        usage_percent: total_usage / cpu_count as f32,
    };

    cpu
}

// Collects all current system information into one snapshot.
fn collect_snapshot() -> SystemSnapshot {
    let snapshot = SystemSnapshot {
        memory: collect_memory(),
        system: collect_system_info(),
        cpu: collect_cpu_info(),
        disk: collect_disks(),
        processes: collect_processes(),
    };

    snapshot
}

// Prints the information contained in a system snapshot.
fn print_snapshot(snapshot: &SystemSnapshot) {
    // Convert total and used RAM from bytes to GiB.
    let t_gb = snapshot.memory.total_bytes as f64 / 1024_f64.powi(3);
    let u_gb = snapshot.memory.used_bytes as f64 / 1024_f64.powi(3);

    // Extract frequently used snapshot fields.
    let hostname = &snapshot.system.hostname;
    let os_name = &snapshot.system.os_name;
    let kernel_version = &snapshot.system.kernel_version;
    let uptime = snapshot.system.uptime_seconds;
    let hours = uptime / 3600;
    let minutes = (uptime % 3600) / 60;
    let cpu_count = snapshot.cpu.logical_cpu_count;
    let cpu_usage = snapshot.cpu.usage_percent;
    let processes = &snapshot.processes;

    // Create a second process list sorted by memory usage.
    let memory_sorted = sort_by_memory(&snapshot.processes);

    // Print memory, system, uptime, and CPU information.
    println!("Total RAM: {:.2} GiB", t_gb);
    println!("Used RAM: {:.2} GiB ", u_gb);
    println!("Host name: {:?}", hostname);
    println!("OS name: {:?}", os_name);
    println!("Kernel Version: {:?}", kernel_version);
    println!("Uptime: {} hrs {} mins", hours, minutes);
    println!("Total number of cpus: {}", cpu_count);
    println!("Percentage of CPU used: {}%", cpu_usage);

    // Print information for each detected disk.
    for disk in &snapshot.disk {
        let total_gib = disk.total_bytes as f64 / 1024_f64.powi(3);
        let available_gib = disk.available_bytes as f64 / 1024_f64.powi(3);

        println!("Disk: {}", disk.name);
        println!("Total: {:.2} GiB", total_gib);
        println!("Available: {:.2} GiB", available_gib);
    }

    // Display the process list in CPU order and then memory order.
    println!("Processes found: {}", processes.len());
    print_processes(processes);
    print_processes(&memory_sorted);

    // Display totals for the collected processes.
    summarize_processes(&snapshot.processes);
}

// Calculates CPU and memory statistics across multiple samples.
fn calculate_resource_summary(
    cpu_usage: &[f32],
    memory_used_history: &[u64],
    memory_usage_percentage: &[f64],
) -> ResourceSummary {
    // Calculate average CPU usage if samples are available.
    let average_cpu = if !cpu_usage.is_empty() {
        Some(cpu_usage.iter().sum::<f32>() / cpu_usage.len() as f32)
    } else {
        None
    };

    // Find the minimum and maximum CPU readings.
    let min_cpu = cpu_usage.iter().copied().reduce(f32::min);
    let max_cpu = cpu_usage.iter().copied().reduce(f32::max);

    // Calculate average RAM usage, converting bytes to MiB.
    let average_memory_mib = if !memory_used_history.is_empty() {
        let total: f64 = memory_used_history
            .iter()
            .map(|&bytes| bytes as f64)
            .sum();

        Some(total / memory_used_history.len() as f64 / 1024_f64.powi(2))
    } else {
        None
    };

    // Find the minimum RAM usage and convert it to MiB.
    let min_memory_mib = memory_used_history
        .iter()
        .copied()
        .min()
        .map(|bytes| bytes as f64 / 1024_f64.powi(2));

    // Find the maximum RAM usage and convert it to MiB.
    let max_memory_mib = memory_used_history
        .iter()
        .copied()
        .max()
        .map(|bytes| bytes as f64 / 1024_f64.powi(2));

    // Calculate average RAM usage percentage using valid samples.
    let average_memory_percentage = if !memory_usage_percentage.is_empty() {
        Some(
            memory_usage_percentage.iter().sum::<f64>()
                / memory_usage_percentage.len() as f64,
        )
    } else {
        None
    };

    // Package all calculated statistics into the summary.
    ResourceSummary {
        samples_collected: cpu_usage.len(),
        average_cpu,
        min_cpu,
        max_cpu,
        average_memory_mib,
        min_memory_mib,
        max_memory_mib,
        average_memory_percentage,
    }
}

// Prints the final CPU and RAM statistics.
fn print_resource_summary(summary: &ResourceSummary) {
    println!("\n========== RESOURCE SUMMARY ==========");
    println!("Samples collected: {}", summary.samples_collected);

    // Print the average CPU usage or explain that no data exists.
    match summary.average_cpu {
        Some(value) => println!("Average CPU Usage: {:.2}%", value),
        None => println!("Average CPU Usage: No samples"),
    }

    // Print the minimum CPU usage.
    match summary.min_cpu {
        Some(value) => println!("Minimum CPU Usage: {:.2}%", value),
        None => println!("Minimum CPU Usage: No samples"),
    }

    // Print the maximum CPU usage.
    match summary.max_cpu {
        Some(value) => println!("Maximum CPU Usage: {:.2}%", value),
        None => println!("Maximum CPU Usage: No samples"),
    }

    // Print the average RAM usage in MiB.
    match summary.average_memory_mib {
        Some(value) => println!("Average RAM Usage: {:.2} MiB", value),
        None => println!("Average RAM Usage: No samples"),
    }

    // Print the minimum RAM usage in MiB.
    match summary.min_memory_mib {
        Some(value) => println!("Minimum RAM Usage: {:.2} MiB", value),
        None => println!("Minimum RAM Usage: No samples"),
    }

    // Print the maximum RAM usage in MiB.
    match summary.max_memory_mib {
        Some(value) => println!("Maximum RAM Usage: {:.2} MiB", value),
        None => println!("Maximum RAM Usage: No samples"),
    }

    // Print the average RAM usage percentage.
    match summary.average_memory_percentage {
        Some(value) => {
            println!("Average RAM Usage Percentage: {:.2}%", value)
        }
        None => println!("Average RAM Usage Percentage: No valid samples"),
    }
}

// Runs repeated system monitoring and returns the final summary.
fn run_monitoring() -> ResourceSummary {
    // Configure how many samples to collect and the sampling interval.
    let sample_limit: usize = 10;
    let sample_interval = std::time::Duration::from_secs(1);

    // Configure the thresholds and consecutive readings for alerts.
    let alert_config = AlertConfig {
        max_cpu_per: 80.0,
        max_ram_per: 85.0,
        consecutive_alert: 3,
    };

    // Track consecutive high CPU and RAM readings independently.
    let mut cpu_counter: u32 = 0;
    let mut ram_counter: u32 = 0;
    let mut cpu_alert_triggered:bool = false;
    let mut ram_alert_triggered:bool = false;

    // Store resource alerts once the detection logic is implemented.
    let mut alert_collection: Vec<ResourceAlert> = Vec::new();

    // Record the start time to schedule samples and measure duration.
    let monitoring_start = Instant::now();

    // Store the history needed to calculate the final summary.
    let mut cpu_usage: Vec<f32> = Vec::new();
    let mut memory_used_history: Vec<u64> = Vec::new();
    let mut memory_usage_percentage: Vec<f64> = Vec::new();

    // Collect and process one system snapshot per iteration.
    for iteration in 0..sample_limit {
        let snapshot = collect_snapshot();

        // Increase the CPU counter when usage exceeds its threshold.
        // Reset the counter when CPU usage returns to or below it.
        if snapshot.cpu.usage_percent > alert_config.max_cpu_per {
            cpu_counter += 1;
        } else {
            cpu_counter = 0;
            cpu_alert_triggered=false;
        }

        // Save CPU usage and RAM usage in bytes for summary calculations.
        cpu_usage.push(snapshot.cpu.usage_percent);
        memory_used_history.push(snapshot.memory.used_bytes);

        // Calculate RAM usage percentage only when total RAM is valid.
        if snapshot.memory.total_bytes > 0 {
            let percentage = snapshot.memory.used_bytes as f64
                / snapshot.memory.total_bytes as f64
                * 100.0;

            // Increase the RAM counter when usage exceeds its threshold.
            // Reset it when RAM usage returns to or below the threshold.
            if percentage as f32 > alert_config.max_ram_per {
                ram_counter += 1;
            } else {
                ram_counter = 0;
                ram_alert_triggered=false;
            }

            // Save valid RAM percentages for the final summary.
            memory_usage_percentage.push(percentage);
        } else {
            // An invalid RAM sample breaks the consecutive-high streak.
            ram_counter = 0;
            ram_alert_triggered=false;
        }
        // CPU Alert handling
        if cpu_counter >= alert_config.consecutive_alert && !cpu_alert_triggered{
    	  alert_collection.push(ResourceAlert{
    	  	trigger_source:ResourceType::CPU,
    	  	usage_per:snapshot.cpu.usage_percent,
    	  	exceeded_threshold:alert_config.max_cpu_per,
    	  });
    	  cpu_alert_triggered=true;
    	  }
    	  // RAM Alert handling
    	  if ram_counter >= alert_config.consecutive_alert && !ram_alert_triggered{
    	      	  alert_collection.push(ResourceAlert{
    	      	  	trigger_source:ResourceType::RAM,
    	      	  	usage_per: snapshot.memory.used_bytes as f32
                / snapshot.memory.total_bytes as f32
                * 100.0,
    	      	  	exceeded_threshold:alert_config.max_ram_per,
    	      	  });
    	      	  ram_alert_triggered=true;
    	  
    }
        // Display the current snapshot.
        print_snapshot(&snapshot);

        // Calculate the deadline for the next scheduled sample.
        let next_deadline = sample_interval * (iteration as u32 + 1);
        let elapsed = monitoring_start.elapsed();

        // Sleep only if the next deadline hasn't already passed.
        if elapsed < next_deadline {
            std::thread::sleep(next_deadline - elapsed);
        }
    }
   

    // Display the total time spent monitoring.
    println!(
        "Total monitoring duration: {:.2} seconds",
        monitoring_start.elapsed().as_secs_f64()
    );

    // Calculate and return the final resource statistics.
    calculate_resource_summary(
        &cpu_usage,
        &memory_used_history,
        &memory_usage_percentage,
    )
}

// Entry point of the NEXUS program.

fn main() {
    // Run monitoring and collect the resulting summary.
    let summary = run_monitoring();

    // Print the summary after monitoring finishes.
    print_resource_summary(&summary);
}
