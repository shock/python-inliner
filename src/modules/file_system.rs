use std::fs;
use std::path::{Path, PathBuf};
use std::io;

pub trait FileSystem {

    #[allow(unused)]
    fn canonicalize<P: AsRef<Path>>(&self, path: P) -> io::Result<PathBuf>;

    #[allow(unused)]
    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&mut self, path: P, contents: C) -> io::Result<()>;

    #[allow(unused)]
    fn read_to_string<P: AsRef<Path>>(&mut self, path: P) -> io::Result<String>;

    #[allow(unused)]
    fn mkdir_p<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()>;

    #[allow(unused)]
    fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()>;

    #[allow(unused)]
    fn remove_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()>;

    #[allow(unused)]
    fn is_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<bool>;

    #[allow(unused)]
    fn is_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<bool>;

    #[allow(unused)]
    fn exists<P: AsRef<Path>>(&mut self, path: P) -> io::Result<bool>;
}

pub struct RealFileSystem {
    #[allow(unused)]
    current_dir: PathBuf,
}

impl RealFileSystem {
    pub fn new(current_dir: PathBuf) -> Self {
        RealFileSystem {
            current_dir: current_dir,
        }
    }
}

impl FileSystem for RealFileSystem {
    fn canonicalize<P: AsRef<Path>>(&self, path: P) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&mut self, path: P, contents: C) -> io::Result<()> {
        fs::write(path, contents)
    }

    fn read_to_string<P: AsRef<Path>>(&mut self, path: P) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn mkdir_p<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        fs::remove_dir(path)
    }

    fn is_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<bool> {
        fs::metadata(path).map(|m| m.is_file())
    }

    fn is_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<bool> {
        fs::metadata(path).map(|m| m.is_dir())
    }

    fn exists<P: AsRef<Path>>(&mut self, path: P) -> io::Result<bool> {
        fs::metadata(path).map(|m| m.is_file() || m.is_dir())
    }
}
