//! ML-DSA-65 keypair generator for saorsa-node release signing.
//!
//! This utility generates a new ML-DSA-65 keypair and outputs:
//! - Public key as a Rust array literal (for embedding in signature.rs)
//! - Private key saved to a file (for CI/CD signing)
//!
//! Usage:
//!   cargo run --bin saorsa-keygen [output-dir]

use saorsa_pqc::api::sig::ml_dsa_65;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    println!("ML-DSA-65 Keypair Generator for saorsa-node releases\n");

    // Get output directory from args or use current directory
    let args: Vec<String> = env::args().collect();
    let output_dir = if args.len() > 1 {
        Path::new(&args[1]).to_path_buf()
    } else {
        env::current_dir().expect("Failed to get current directory")
    };

    // Create output directory if it doesn't exist
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    println!("Generating ML-DSA-65 keypair...");

    // Generate keypair
    let dsa = ml_dsa_65();
    let (public_key, secret_key) = dsa.generate_keypair().expect("Failed to generate keypair");

    let pk_bytes = public_key.to_bytes();
    let sk_bytes = secret_key.to_bytes();

    println!("  Public key size: {} bytes", pk_bytes.len());
    println!("  Secret key size: {} bytes", sk_bytes.len());

    // Save secret key to file (KEEP THIS SECURE!)
    let sk_path = output_dir.join("release-signing-key.secret");
    fs::write(&sk_path, sk_bytes).expect("Failed to write secret key");
    println!("\nSecret key saved to: {}", sk_path.display());
    println!("  WARNING: Keep this file secure! It's needed for signing releases.");

    // Save public key to file
    let pk_path = output_dir.join("release-signing-key.pub");
    fs::write(&pk_path, &pk_bytes).expect("Failed to write public key");
    println!("Public key saved to: {}", pk_path.display());

    // Generate Rust code for embedding
    let rust_code_path = output_dir.join("release_key_embed.rs");
    let mut rust_file = fs::File::create(&rust_code_path).expect("Failed to create Rust file");

    writeln!(rust_file, "/// Embedded release signing public key (ML-DSA-65).").unwrap();
    writeln!(rust_file, "///").unwrap();
    writeln!(
        rust_file,
        "/// This key is used to verify signatures on released binaries."
    )
    .unwrap();
    writeln!(
        rust_file,
        "/// The corresponding private key is held by authorized release signers."
    )
    .unwrap();
    writeln!(
        rust_file,
        "/// Generated: {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    )
    .unwrap();
    writeln!(rust_file, "const RELEASE_SIGNING_KEY: &[u8] = &[").unwrap();

    // Write bytes in rows of 16 for readability
    for (i, byte) in pk_bytes.iter().enumerate() {
        if i % 16 == 0 {
            write!(rust_file, "    ").unwrap();
        }
        write!(rust_file, "0x{byte:02x},").unwrap();
        if i % 16 == 15 {
            writeln!(rust_file).unwrap();
        } else {
            write!(rust_file, " ").unwrap();
        }
    }

    // Handle last line if not complete
    if pk_bytes.len() % 16 != 0 {
        writeln!(rust_file).unwrap();
    }

    writeln!(rust_file, "];").unwrap();

    println!("Rust embed code saved to: {}", rust_code_path.display());

    // Also print to stdout for convenience
    println!("\n--- Rust code for signature.rs ---\n");
    println!("const RELEASE_SIGNING_KEY: &[u8] = &[");
    for (i, byte) in pk_bytes.iter().enumerate() {
        if i % 16 == 0 {
            print!("    ");
        }
        print!("0x{byte:02x},");
        if i % 16 == 15 {
            println!();
        } else {
            print!(" ");
        }
    }
    if pk_bytes.len() % 16 != 0 {
        println!();
    }
    println!("];");

    println!("\n--- End of Rust code ---");
    println!("\nDone! Copy the above code to src/upgrade/signature.rs");
}
