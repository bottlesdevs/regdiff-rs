use regdiff_rs::prelude::{Diff, Hive, Registry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o_reg = Registry::try_from("./registries/old.reg", Hive::LocalMachine)?;
    let n_reg = Registry::try_from("./registries/new.reg", Hive::LocalMachine)?;

    let diff = Registry::diff(&o_reg, &n_reg);

    diff.serialize_file("patch.reg")?;
    Ok(())
}
