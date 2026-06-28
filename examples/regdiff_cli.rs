// CLI around regdiff-rs for the capture/replay loop:
//   regdiff_cli diff  <old.reg>  <new.reg>   <out-patch.reg>   [hive]
//   regdiff_cli apply <base.reg> <patch.reg> <out-merged.reg>  [hive]
// hive is hklm (default) or hkcu.
use regdiff_rs::prelude::{apply_files, Diff, Hive, Registry};
use std::env;

fn hive(arg: Option<&String>) -> Hive {
    match arg.map(String::as_str) {
        Some("hkcu") | Some("HKCU") => Hive::CurrentUser,
        _ => Hive::LocalMachine,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = env::args().collect();
    if a.len() < 5 {
        eprintln!("usage:");
        eprintln!("  regdiff_cli diff  <old.reg>  <new.reg>   <out-patch.reg>  [hklm|hkcu]");
        eprintln!("  regdiff_cli apply <base.reg> <patch.reg> <out-merged.reg> [hklm|hkcu]");
        std::process::exit(2);
    }
    let h = hive(a.get(5));
    match a[1].as_str() {
        "diff" => {
            let old = Registry::try_from(a[2].as_str(), h)?;
            let new = Registry::try_from(a[3].as_str(), h)?;
            Registry::diff(&old, &new).serialize_file(a[4].as_str())?;
        }
        "apply" => {
            apply_files(a[2].as_str(), a[3].as_str(), a[4].as_str(), h)?;
        }
        other => {
            eprintln!("unknown command: {other}");
            std::process::exit(2);
        }
    }
    Ok(())
}
