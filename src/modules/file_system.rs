use std::fs;
use std::path::{Path, PathBuf};
use std::io;

pub trait FileSystemLike {
    fn canonicalize<P>(path: P) -> io::Result<PathBuf>;
    fn write<P, C>(path: P, contents: C) -> io::Result<()>;
    fn read_to_string<P>(path: P) -> io::Result<String>;
    fn mkdir_p<P>(path: P) -> io::Result<()>;
    fn remove_file<P>(path: P) -> io::Result<()>;
    fn remove_dir<P>(path: P) -> io::Result<()>;
}

pub struct FileSystem {}

impl FileSystemLike for FileSystem {
    fn canonicalize<P>(path: P) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }

    fn write<P, C>(path: P, contents: C) -> io::Result<()> {
        fs::write(path, contents)
    }

    fn read_to_string<P>(path: P) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn mkdir_p<P>(path: P) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn remove_file<P>(path: P) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir<P>(path: P) -> io::Result<()> {
        fs::remove_dir(path)
    }
}