//! Manual test for F020 Clean Uninstall
//!
//! Run with: cargo run --example test_uninstall

use aegis_app::uninstall::{get_confirmation_text, UninstallManager, UninstallPaths};
use aegis_storage::Database;

fn main() {
    println!("=== F020 Clean Uninstall Test ===\n");

    // 1. Show what paths would be deleted
    println!("1. Data Paths Discovery:");
    match UninstallPaths::default_paths() {
        Some(paths) => {
            println!("   Data dir:  {}", paths.data_dir.display());
            println!("   CA dir:    {}", paths.ca_dir.display());
            println!("   Database:  {}", paths.database.display());
            println!("\n   Confirmation text:");
            for line in get_confirmation_text(&paths).lines() {
                println!("   {}", line);
            }
        }
        None => println!("   Could not determine paths"),
    }

    // 2. Show OS-specific CA removal instructions
    println!("\n2. CA Removal Instructions:");
    for line in UninstallManager::get_ca_removal_instructions().lines() {
        println!("   {}", line);
    }

    // 3. Test with in-memory database (safe - no real data deleted)
    println!("\n3. Auth & Uninstall Flow Test (in-memory):");

    let db = Database::in_memory().expect("Failed to create test database");

    // Set up auth
    let auth = aegis_core::auth::AuthManager::new();
    let hash = auth.hash_password("test123").unwrap();
    db.set_password_hash(&hash).unwrap();

    // Log a test event
    db.log_event(
        "test prompt for export",
        None,
        None,
        aegis_storage::models::Action::Allowed,
        Some("test".to_string()),
    )
    .unwrap();

    let mut manager = UninstallManager::new(db);

    // Test wrong password
    println!("   Testing wrong password...");
    let result = manager.verify_auth("wrong").unwrap();
    println!("   Wrong password result: {} (expected: false)", result);

    // Test correct password
    println!("   Testing correct password...");
    let result = manager.verify_auth("test123").unwrap();
    println!("   Correct password result: {} (expected: true)", result);
    println!("   Is authenticated: {}", manager.is_authenticated());

    // Test export (to temp file)
    println!("\n4. Log Export Test:");
    let export_path = std::env::temp_dir().join("aegis_test_export.csv");
    match manager.export_logs(&export_path) {
        Ok(count) => {
            println!("   Exported {} events to {}", count, export_path.display());
            if let Ok(content) = std::fs::read_to_string(&export_path) {
                println!("   CSV content:");
                for line in content.lines().take(5) {
                    println!("   {}", line);
                }
            }
            // Clean up
            let _ = std::fs::remove_file(&export_path);
        }
        Err(e) => println!("   Export failed: {}", e),
    }

    // Note: We don't actually perform uninstall on real paths
    println!("\n5. Uninstall (would delete real data - skipped for safety)");
    println!("   To test actual deletion, modify this script to use real paths");

    println!("\n=== Test Complete ===");
}
