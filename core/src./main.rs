struct MemoryInfo {
    total: u64,
    used: u64,
}

fn main() {
    let memory = MemoryInfo {
        total: 16,
        used: 9,
    };

    println!("Total RAM: {}", memory.total);
    println!("Used RAM: {}", memory.used);
}
