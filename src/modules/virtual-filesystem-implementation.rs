use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io::{self, ErrorKind};
use std::sync::RwLock;

pub struct VirtualFileSystem {
    root: RwLock<VirtualDirectory>,
    cwd: RwLock<PathBuf>,
}

struct VirtualDirectory {
    contents: HashMap<String, VirtualNode>,
}

struct VirtualFile {
    contents: String,
}

enum VirtualNode {
    Directory(VirtualDirectory),
    File(VirtualFile),
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        VirtualFileSystem {
            root: RwLock::new(VirtualDirectory { contents: HashMap::new() }),
            cwd: RwLock::new(PathBuf::from("/")),
        }
    }

    fn resolve_path<P: AsRef<Path>>(&self, path: P) -> io::Result<Vec<String>> {
        let path = path.as_ref();
        let mut current_path = if path.is_absolute() {
            PathBuf::from("/")
        } else {
            self.cwd.read().unwrap().clone()
        };

        let mut components = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::RootDir => current_path = PathBuf::from("/"),
                std::path::Component::ParentDir => {
                    if current_path.pop() == false && current_path != Path::new("/") {
                        return Err(io::Error::new(ErrorKind::NotFound, "Path not found"));
                    }
                },
                std::path::Component::Normal(os_str) => {
                    let segment = os_str.to_str().ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "Invalid path"))?;
                    current_path.push(segment);
                    components.push(segment.to_string());
                },
                _ => {}
            }
        }
        Ok(components)
    }

    fn get_node(&self, path: &[String]) -> io::Result<&VirtualNode> {
        let root = self.root.read().unwrap();
        let mut current_node = &VirtualNode::Directory(root);

        for segment in path {
            match current_node {
                VirtualNode::Directory(dir) => {
                    current_node = dir.contents.get(segment)
                        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "Path not found"))?;
                },
                VirtualNode::File(_) => return Err(io::Error::new(ErrorKind::NotADirectory, "Not a directory")),
            }
        }
        Ok(current_node)
    }

    fn get_node_mut(&mut self, path: &[String]) -> io::Result<&mut VirtualNode> {
        let mut root = self.root.write().unwrap();
        let mut current_node = &mut VirtualNode::Directory(root);

        for segment in path {
            match current_node {
                VirtualNode::Directory(dir) => {
                    current_node = dir.contents.get_mut(segment)
                        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "Path not found"))?;
                },
                VirtualNode::File(_) => return Err(io::Error::new(ErrorKind::NotADirectory, "Not a directory")),
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

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&self, path: P, contents: C) -> io::Result<()> {
        let components = self.resolve_path(path)?;
        let parent_components = &components[..components.len() - 1];
        let filename = components.last().ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "Invalid path"))?;

        let mut root = self.root.write().unwrap();
        let mut current_dir = &mut root;

        for component in parent_components {
            current_dir = match current_dir.contents.entry(component.to_string()) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(VirtualNode::Directory(VirtualDirectory { contents: HashMap::new() }))
                },
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            };

            if let VirtualNode::Directory(dir) = current_dir {
                current_dir = dir;
            } else {
                return Err(io::Error::new(ErrorKind::NotADirectory, "Not a directory"));
            }
        }

        let contents_str = String::from_utf8_lossy(contents.as_ref()).into_owned();
        current_dir.contents.insert(filename.to_string(), VirtualNode::File(VirtualFile { contents: contents_str }));
        Ok(())
    }

    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> io::Result<String> {
        let components = self.resolve_path(path)?;
        match self.get_node(&components)? {
            VirtualNode::File(file) => Ok(file.contents.clone()),
            VirtualNode::Directory(_) => Err(io::Error::new(ErrorKind::IsADirectory, "Is a directory")),
        }
    }

    fn mkdir_p<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let components = self.resolve_path(path)?;
        let mut root = self.root.write().unwrap();
        let mut current_dir = &mut root;

        for component in components {
            current_dir = match current_dir.contents.entry(component) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(VirtualNode::Directory(VirtualDirectory { contents: HashMap::new() }))
                },
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            };

            if let VirtualNode::Directory(dir) = current_dir {
                current_dir = dir;
            } else {
                return Err(io::Error::new(ErrorKind::AlreadyExists, "File exists"));
            }
        }
        Ok(())
    }

    fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let components = self.resolve_path(path)?;
        let parent_components = &components[..components.len() - 1];
        let filename = components.last().ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "Invalid path"))?;

        let mut root = self.root.write().unwrap();
        let mut current_dir = &mut root;

        for component in parent_components {
            match current_dir.contents.get_mut(component) {
                Some(VirtualNode::Directory(dir)) => current_dir = dir,
                _ => return Err(io::Error::new(ErrorKind::NotFound, "Path not found")),
            }
        }

        match current_dir.contents.remove(filename) {
            Some(VirtualNode::File(_)) => Ok(()),
            Some(VirtualNode::Directory(_)) => Err(io::Error::new(ErrorKind::IsADirectory, "Is a directory")),
            None => Err(io::Error::new(ErrorKind::NotFound, "File not found")),
        }
    }

    fn remove_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let components = self.resolve_path(path)?;
        let parent_components = &components[..components.len() - 1];
        let dirname = components.last().ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "Invalid path"))?;

        let mut root = self.root.write().unwrap();
        let mut current_dir = &mut root;

        for component in parent_components {
            match current_dir.contents.get_mut(component) {
                Some(VirtualNode::Directory(dir)) => current_dir = dir,
                _ => return Err(io::Error::new(ErrorKind::NotFound, "Path not found")),
            }
        }

        match current_dir.contents.get(dirname) {
            Some(VirtualNode::Directory(dir)) if dir.contents.is_empty() => {
                current_dir.contents.remove(dirname);
                Ok(())
            },
            Some(VirtualNode::Directory(_)) => Err(io::Error::new(ErrorKind::Other, "Directory not empty")),
            Some(VirtualNode::File(_)) => Err(io::Error::new(ErrorKind::Other, "Not a directory")),
            None => Err(io::Error::new(ErrorKind::NotFound, "Directory not found")),
        }
    }
}
