fn main() {
    let args: Vec<String> = std::env::args().collect();
    let url = args.get(1).map(|s| s.as_str()).unwrap_or("https://google.com");
    
    println!("Bliss: Loading URL -> {}", url);
    bliss::launch_url(url);
}
