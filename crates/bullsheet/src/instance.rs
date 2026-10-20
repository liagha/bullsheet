pub struct Instance {
    file: std::fs::File,
}

impl Instance {
    pub fn new() -> std::io::Result<Option<Self>> {
        let base = directories::BaseDirs::new().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "no home directory")
        })?;
        let directory = base.data_dir().join("bullsheet");
        std::fs::create_dir_all(&directory)?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(directory.join("instance.lock"))?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Self { file })),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(error)) => Err(error),
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
