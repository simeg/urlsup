use std::io::Write;
use std::path::Path;
use std::{fs, io};

pub trait WriteToFile {
    fn write_to_file(&self, path: &Path, data: String) -> io::Result<()>;
}

#[derive(Default)]
pub struct Writer;

impl WriteToFile for Writer {
    fn write_to_file(&self, path: &Path, data: String) -> io::Result<()> {
        let mut file = fs::OpenOptions::new().write(true).open(path).unwrap();

        file.write_all(format!("{}\n", data).as_bytes())?;

        Ok(())
    }
}
