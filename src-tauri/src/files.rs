use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};


use crate::cv::*;

/// Write next to the destination, then rename so a failed write leaves the
/// existing file untouched.
pub(crate) fn save_atomically(path: &Path, bytes: &[u8], lang: Language) -> Result<(), String> {
    let invalid = || lang.pick("invalid destination path", "tujuan tidak valid");
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).ok_or_else(invalid)?;
    let name = path.file_name().ok_or_else(invalid)?.to_string_lossy();
    let tmp = dir.join(format!(".{}.tmp-{}", name, std::process::id()));
    if let Err(e) = fs::write(&tmp, bytes) {
        let _ = fs::remove_file(&tmp); // never leave a partial snapshot behind
        return Err(lang.pick(format!("writing the file failed: {e}"), format!("penulisan berkas gagal: {e}")));
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(lang.pick(
            format!("could not replace the destination: {e}"),
            format!("tidak dapat mengganti tujuan: {e}"),
        ));
    }
    Ok(())
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}
