use std::process::Command;
use std::env;

fn main() {
    println!("cargo:rerun-if-changed=schema/chess.fbs");
    
    let out_dir = env::var("OUT_DIR").unwrap();
    let output_path = format!("{}/chess.rs", out_dir);
    
    let output = Command::new("planus")
        .args(["rust", "-o", &output_path, "schema/chess.fbs"])
        .output()
        .expect("Failed to execute planus command");
    
    if !output.status.success() {
        panic!("planus failed: {}", String::from_utf8_lossy(&output.stderr));
    }
}
