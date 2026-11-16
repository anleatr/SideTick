use crate::model::State;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

pub const ENVIRONMENT: &str = if cfg!(feature = "production") {
    "production"
} else {
    "development"
};

pub fn state_path() -> io::Result<PathBuf> {
    let root = crate::windows::data_root()?
        .join("com.ayangweb.bongo-cat")
        .join(ENVIRONMENT);
    std::fs::create_dir_all(&root)?;
    Ok(root.join("personal-state.json"))
}
pub fn load(path: &Path, today: i64) -> io::Result<State> {
    let mut state = match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<State>(&bytes).map_err(io::Error::other)?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => State::new(today),
        Err(e) => return Err(e),
    };
    if !state.validate() {
        return Err(io::Error::other("invalid personal state"));
    }
    state.update_day(today);
    Ok(state)
}
pub fn save(path: &Path, state: &State) -> io::Result<()> {
    let mut file = atomic_write_file::AtomicWriteFile::open(path)?;
    file.write_all(&serde_json::to_vec_pretty(state).map_err(io::Error::other)?)?;
    file.sync_all()?;
    file.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_save_restore_and_invalid_file_is_preserved() {
        let dir = std::env::temp_dir().join(format!("bongocat-state-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let mut state = State::new(100);
        state.total_millis = 12345;
        save(&path, &state).unwrap();
        let loaded = load(&path, 103).unwrap();
        assert_eq!(loaded.total_millis, 12345);
        assert_eq!(loaded.days, 3);
        std::fs::write(&path, b"invalid").unwrap();
        assert!(load(&path, 104).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"invalid");
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }
}
