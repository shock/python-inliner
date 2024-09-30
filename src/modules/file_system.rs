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
    fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<Vec<PathBuf>>;

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

    fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<Vec<PathBuf>> {
        // map the read_dir result to a vector of PathBuf
        let read_dir = fs::read_dir(path)?;
        let mut paths = Vec::new();
        for entry in read_dir {
            let entry = entry?;
            paths.push(entry.path());
        }
        Ok(paths)
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
        match fs::metadata(path) {
            Ok(m) => Ok(m.is_file()),
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(e)
                }
            },
        }
    }

    fn is_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<bool> {
        match fs::metadata(path) {
            Ok(m) => Ok(m.is_dir()),
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(e)
                }
            },
        }
    }

    fn exists<P: AsRef<Path>>(&mut self, path: P) -> io::Result<bool> {
        match fs::metadata(path) {
            Ok(m) => Ok(m.is_file() || m.is_dir()),
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(e)
                }
            },
        }
    }
}
