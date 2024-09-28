use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io;
use crate::modules::file_system::FileSystem;

pub struct VirtualFileSystem {
    root: VirtualNode,
    cwd: PathBuf,
}

#[derive(Debug, Clone)]
struct VirtualDirectory {
    contents: HashMap<String, VirtualNode>,
}

#[derive(Debug, Clone)]
struct VirtualFile {
    contents: String,
}

#[derive(Debug, Clone)]
enum VirtualNode {
    Directory(VirtualDirectory),
    File(VirtualFile),
}

impl VirtualFileSystem {
    #[allow(unused)]
    pub fn new() -> Self {
        VirtualFileSystem {
            root: VirtualNode::Directory(VirtualDirectory { contents: HashMap::new() }),
            cwd: PathBuf::from("/"),
        }
    }

    fn resolve_path<P: AsRef<Path>>(&self, path: P) -> io::Result<Vec<String>> {
        let path = path.as_ref();
        let mut current_path = if path.is_absolute() {
            PathBuf::from("/")
        } else {
            self.cwd.clone()
        };

        let mut components = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::RootDir => current_path = PathBuf::from("/"),
                std::path::Component::ParentDir => {
                    if current_path.pop() == false && current_path != Path::new("/") {
                        return Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"));
                    }
                },
                std::path::Component::Normal(os_str) => {
                    let segment = os_str.to_str().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?;
                    current_path.push(segment);
                    components.push(segment.to_string());
                },
                _ => {}
            }
        }
        Ok(components)
    }

    fn get_node(&mut self, path: &[String]) -> io::Result<&VirtualNode> {
        let mut current_node = &self.root;

        for segment in path {
            match current_node {
                VirtualNode::Directory(dir) => {
                    let node = dir.contents.get(segment);
                    if let Some(node) = node {
                        current_node = &node;
                    } else {
                        return Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"));
                    }
                },
                VirtualNode::File(_) => return Err(io::Error::new(io::ErrorKind::Other, "Not a directory")),
            }
        }
        Ok(current_node)
    }

    fn get_node_mut(&mut self, path: &[String]) -> io::Result<&mut VirtualNode> {
        let mut current_node = &mut self.root;

        for segment in path {
            match current_node {
                VirtualNode::Directory(dir) => {
                    current_node = dir.contents.get_mut(segment)
                        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Path not found"))?;
                },
                VirtualNode::File(_) => return Err(io::Error::new(io::ErrorKind::Other, "Not a directory")),
            }
        }
        Ok(current_node)
    }

}

impl FileSystem for VirtualFileSystem {
    fn canonicalize<P: AsRef<Path>>(&self, path: P) -> io::Result<PathBuf> {
        let components = self.resolve_path(path)?;
        let mut canonical_path = PathBuf::from("/");
        for component in components {
            canonical_path.push(component);
        }
        Ok(canonical_path)
    }

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&mut self, path: P, contents: C) -> io::Result<()> {
        let components = self.resolve_path(path)?;
        let parent_components = &components[..components.len() - 1];
        let filename = components.last().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?;

        let mut parent_node = self.get_node_mut(parent_components)?;

        if let VirtualNode::Directory(dir) = &mut parent_node {
            let contents_str = String::from_utf8_lossy(contents.as_ref()).into_owned();
            dir.contents.insert(filename.to_string(), VirtualNode::File(VirtualFile { contents: contents_str }));
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Not a directory"))
        }
    }

    fn read_to_string<P: AsRef<Path>>(&mut self, path: P) -> io::Result<String> {
        let components = self.resolve_path(path)?;
        match self.get_node(&components)? {
            VirtualNode::File(file) => Ok(file.contents.clone()),
            VirtualNode::Directory(_) => Err(io::Error::new(io::ErrorKind::Other, "Is a directory")),
        }
    }

    fn mkdir_p<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let components = self.resolve_path(path)?;
        let mut current_node = &mut self.root;

        for component in components {
            match current_node {
                VirtualNode::Directory(dir) => {
                    let entry = dir.contents.entry(component.to_string());
                    current_node = match entry {
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            let new_dir = VirtualDirectory { contents: HashMap::new() };
                            let node = VirtualNode::Directory(new_dir);
                            entry.insert(node)
                        },
                        std::collections::hash_map::Entry::Occupied(entry) => {
                            entry.into_mut()
                        },
                    };
                },
                VirtualNode::File(_) => return Err(io::Error::new(io::ErrorKind::Other, "Not a directory")),
            }

            if let VirtualNode::File(_) = current_node {
                return Err(io::Error::new(io::ErrorKind::AlreadyExists, "File exists"));
            }
        }
        Ok(())
    }

    fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let components = self.resolve_path(path)?;
        let parent_components = &components[..components.len() - 1];
        let filename = components.last().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?;

        let mut parent_node = self.get_node_mut(parent_components)?;

        if let VirtualNode::Directory(dir) = &mut parent_node {
            match dir.contents.get_mut(filename) {
                Some(VirtualNode::File(_)) => Ok(()),
                Some(VirtualNode::Directory(_)) => Err(io::Error::new(io::ErrorKind::Other, "Is a directory")),
                None => Err(io::Error::new(io::ErrorKind::NotFound, "File not found")),
            }
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "File not found"))
        }
    }

    fn remove_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let components = self.resolve_path(path)?;
        let parent_components = &components[..components.len() - 1];
        let dirname = components.last().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?;

        let mut parent_node = self.get_node_mut(parent_components)?;

        if let VirtualNode::Directory(parent_dir) = &mut parent_node {
            match parent_dir.contents.get_mut(dirname) {
                Some(VirtualNode::Directory(dir)) if dir.contents.is_empty() => {
                    parent_dir.contents.remove(dirname);
                    Ok(())
                },
                Some(VirtualNode::Directory(_)) => Err(io::Error::new(io::ErrorKind::Other, "Directory not empty")),
                Some(VirtualNode::File(_)) => Err(io::Error::new(io::ErrorKind::Other, "Not a directory")),
                None => Err(io::Error::new(io::ErrorKind::NotFound, "Directory not found")),
            }
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "File not found"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_filesystem() {
        let mut fs = VirtualFileSystem::new();
        fs.mkdir_p("/test").unwrap();
        fs.mkdir_p("/test/dir1").unwrap();
        fs.mkdir_p("/test/dir2").unwrap();
        fs.write("/test/file1", "Hello").unwrap();
        fs.write("/test/dir1/file2", "World").unwrap();
        fs.write("/test/dir2/file3", "!").unwrap();

        assert_eq!(fs.read_to_string("/test/file1").unwrap(), "Hello");
        assert_eq!(fs.read_to_string("/test/dir1/file2").unwrap(), "World");
        assert_eq!(fs.read_to_string("/test/dir2/file3").unwrap(), "!");
    }
}