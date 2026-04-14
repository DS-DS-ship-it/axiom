use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::Path,
};

use anyhow::Result;

use crate::{
    commit::{apply_commit_certificate, CommitCertificate},
    state::ChainState,
};

pub fn append_certificate(path: impl AsRef<Path>, cert: &CommitCertificate) -> Result<()> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, cert)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

pub fn read_certificates(path: impl AsRef<Path>) -> Result<Vec<CommitCertificate>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(vec![]);
    }

    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut certs = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        certs.push(serde_json::from_str(&line)?);
    }

    Ok(certs)
}

pub fn replay_wal(path: impl AsRef<Path>, state: &mut ChainState) -> Result<usize> {
    replay_wal_from(path, state, 0)
}

pub fn replay_wal_from(
    path: impl AsRef<Path>,
    state: &mut ChainState,
    skip_entries: usize,
) -> Result<usize> {
    let certs = read_certificates(path)?;
    let mut applied = 0usize;

    for cert in certs.into_iter().skip(skip_entries) {
        apply_commit_certificate(&cert, state)?;
        applied += 1;
    }

    Ok(applied)
}
