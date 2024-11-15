#![windows_subsystem = "windows"]

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tray_item::{IconSource, TrayItem};
use wmi::{COMLibrary, WMIConnection, Variant};
use serde::Deserialize;

#[derive(PartialEq, Clone, Debug)]
enum DefenderStatus {
    Enabled,
    Disabled,
    Unknown,
}

#[derive(Deserialize, Debug)]
struct DefenderProduct {
    #[serde(rename = "RealTimeProtectionEnabled")]
    real_time_protection_enabled: bool,
}

fn create_wmi_connection() -> Result<WMIConnection, Box<dyn std::error::Error>> {
    Ok(WMIConnection::with_namespace_path(
        "ROOT\\Microsoft\\Windows\\Defender",
        COMLibrary::without_security()?,
    )?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create tray icon
    let mut tray = TrayItem::new(
        "Windows Defender Monitor",
        IconSource::Resource("defender_disabled"),
    )?;

    // Create a shared status
    let status = Arc::new(Mutex::new(DefenderStatus::Unknown));

    // Add toggle menu item
    let status_clone = Arc::clone(&status);
    tray.add_menu_item("Toggle Protection", move || {
        if let Ok(current_status) = status_clone.lock() {
            match *current_status {
                DefenderStatus::Enabled => {
                    println!("Attempting to disable protection...");
                    match set_defender_protection(false) {
                        Ok(_) => println!("Command executed successfully"),
                        Err(e) => eprintln!("Failed to disable protection: {}", e),
                    }
                }
                DefenderStatus::Disabled => {
                    println!("Attempting to enable protection...");
                    match set_defender_protection(true) {
                        Ok(_) => println!("Command executed successfully"),
                        Err(e) => eprintln!("Failed to enable protection: {}", e),
                    }
                }
                DefenderStatus::Unknown => {
                    println!("Cannot toggle: status unknown");
                }
            }
        }
    })?;

    // Add status menu item
    let status_clone = Arc::clone(&status);
    tray.add_menu_item("Status", move || {
        if let Ok(status) = status_clone.lock() {
            let message = match *status {
                DefenderStatus::Enabled => "Windows Defender real-time protection is ENABLED",
                DefenderStatus::Disabled => "Windows Defender real-time protection is DISABLED",
                DefenderStatus::Unknown => "Windows Defender status is UNKNOWN",
            };
            println!("{}", message);
        }
    })?;

    // Add quit menu item
    tray.add_menu_item("Quit", || {
        std::process::exit(0);
    })?;

    // Start monitoring loop
    let status_clone = Arc::clone(&status);
    let tray_clone = Arc::new(Mutex::new(tray));
    let tray_clone2 = Arc::clone(&tray_clone);

    thread::spawn(move || {
        loop {
            // Create a new WMI connection for this thread
            let wmi_con = match create_wmi_connection() {
                Ok(con) => con,
                Err(e) => {
                    eprintln!("Failed to create WMI connection: {}", e);
                    thread::sleep(Duration::from_secs(10));
                    continue;
                }
            };

            let new_status = match check_defender_status(&wmi_con) {
                Ok(true) => DefenderStatus::Enabled,
                Ok(false) => DefenderStatus::Disabled,
                Err(e) => {
                    eprintln!("Error checking status: {}", e);
                    DefenderStatus::Unknown
                },
            };

            // Update status
            if let Ok(mut current_status) = status_clone.lock() {
                if *current_status != new_status {
                    println!("Status changed to: {:?}", new_status);
                    *current_status = new_status.clone();

                    // Update tray icon
                    if let Ok(mut tray) = tray_clone2.lock() {
                        let icon = match new_status {
                            DefenderStatus::Enabled => "defender_enabled",
                            DefenderStatus::Disabled => "defender_disabled",
                            DefenderStatus::Unknown => "defender_unknown",
                        };

                        if let Err(e) = tray.set_icon(IconSource::Resource(icon)) {
                            eprintln!("Failed to update tray icon: {}", e);
                        }
                    }
                }
            }

            thread::sleep(Duration::from_secs(10));
        }
    });

    // Keep the main thread alive
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

fn check_defender_status(wmi_con: &WMIConnection) -> Result<bool, Box<dyn std::error::Error>> {
    let query = "SELECT * FROM MSFT_MpComputerStatus";
    let results: Vec<std::collections::HashMap<String, Variant>> = wmi_con.raw_query(query)?;

    if let Some(result) = results.first() {
        if let Some(Variant::Bool(status)) = result.get("RealTimeProtectionEnabled") {
            return Ok(*status);
        }
    }

    Err("Could not get Windows Defender status".into())
}

fn set_defender_protection(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    let script = if enable {
        r#"
Add-Type -AssemblyName System.Windows.Forms;
try {
    Write-Host 'Attempting to enable Real-time protection...';
    Set-MpPreference -DisableRealtimeMonitoring $false;
    Start-Sleep -Seconds 2;
    $status = Get-MpPreference | Select-Object -ExpandProperty DisableRealtimeMonitoring;
    if (!$status) {
        Write-Host 'Successfully enabled Real-time protection';
    } else {
        Write-Host 'Failed to enable Real-time protection';
        exit 1;
    }
} catch {
    Write-Host "Error: $($_.Exception.Message)";
    exit 1;
}"#
    } else {
        r#"
Add-Type -AssemblyName System.Windows.Forms;
try {
    Write-Host 'Attempting to disable Real-time protection...';
    Set-MpPreference -DisableRealtimeMonitoring $true;
    Start-Sleep -Seconds 2;
    $status = Get-MpPreference | Select-Object -ExpandProperty DisableRealtimeMonitoring;
    if ($status) {
        Write-Host 'Successfully disabled Real-time protection';
    } else {
        Write-Host 'Failed to disable Real-time protection';
        exit 1;
    }
} catch {
    Write-Host "Error: $($_.Exception.Message)";
    exit 1;
}"#
    };

    // Save the script to a temporary file
    let script_path = std::env::temp_dir().join("defender_toggle.ps1");
    std::fs::write(&script_path, script)?;

    // Convert the path to a string that PowerShell can understand
    let script_path_str = script_path.to_string_lossy().replace("\\", "\\\\");

    let command = format!(
        "Start-Process powershell -Verb RunAs -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"{script_path_str}\"' -Wait"
    );

    println!("Executing protection toggle command...");
    let output = std::process::Command::new("powershell")
        .args(["-Command", &command])
        .output()?;

    // Clean up the temporary file
    let _ = std::fs::remove_file(script_path);

    println!("Command output:");
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    if output.status.success() {
        Ok(())
    } else {
        Err("Failed to change protection status".into())
    }
}