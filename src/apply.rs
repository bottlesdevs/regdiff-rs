use crate::prelude::Hive;

/// Applies a patch (as produced by [`crate::prelude::Diff`] on registries) onto
/// a base registry in place. The patch keys are hive-prefixed
/// (e.g. `HKEY_LOCAL_MACHINE\...`) while a Wine base registry stores keys
/// relative to its hive, so the prefix is stripped before delegating to
/// regashii's native apply (which merges added keys/values, replaces, and
/// honours key and value deletions).
pub fn apply(base: &mut regashii::Registry, patch: regashii::Registry, hive: Hive) {
    let prefix = format!("{}\\", hive);
    let mut relative = regashii::Registry::new(base.format());
    for (name, key) in patch.keys() {
        let raw = name.raw();
        let stripped = raw.strip_prefix(&prefix).unwrap_or(raw);
        relative = relative.with(regashii::KeyName::new(stripped), key.clone());
    }
    base.apply(relative);
}

/// Convenience wrapper: read a base `*.reg` and a patch `*.reg`, apply the patch,
/// and write the merged result to `out`.
pub fn apply_files<P: AsRef<std::path::Path>>(
    base: P,
    patch: P,
    out: P,
    hive: Hive,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut base = regashii::Registry::deserialize_file(base)?;
    let patch = regashii::Registry::deserialize_file(patch)?;
    apply(&mut base, patch, hive);
    base.serialize_file(out)?;
    Ok(())
}
