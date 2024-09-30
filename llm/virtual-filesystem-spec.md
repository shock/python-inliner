# VirtualFileSystem Design Specification

## 1. Overview
The VirtualFileSystem is an in-memory implementation of the FileSystem trait, designed to mimic the behavior of a real file system for testing purposes.

## 2. Core Components

### 2.1 VirtualFileSystem
- Implements the FileSystem trait
- Maintains a root directory and current working directory
- Provides methods for file system operations

### 2.2 VirtualDirectory
- Represented as a HashMap<String, VirtualNode>
- Keys are filenames (String)
- Values are VirtualNodes (either subdirectories or files)

### 2.3 VirtualFile
- Contains a filename (String)
- Contains file contents (String)

### 2.4 VirtualNode (enum)
- Directory(VirtualDirectory)
- File(VirtualFile)

## 3. Path Handling

### 3.1 Path Parsing
- Split paths by '/' into segments
- Handle both absolute paths (starting with '/') and relative paths
- Treat empty segments (resulting from '//' in the path) as no-ops
- Handle '.' (current directory) and '..' (parent directory) special cases

### 3.2 Path Traversal
- Start from the root directory for absolute paths, or the current working directory for relative paths
- Traverse the directory tree using the parsed segments
- For each segment, look up the corresponding VirtualNode in the current directory's HashMap
- If a segment is '..', move up to the parent directory (if not already at root)
- If a segment is '.', remain in the current directory
- Handle errors for non-existent path components

### 3.3 Path Normalization
- Implement a function to normalize paths (remove redundant '/', '.', and '..' components)
- Use this in the `canonicalize` function and potentially in other operations

### 3.4 Current Working Directory
- Maintain a current working directory state in the VirtualFileSystem
- Update it when operations change the current directory
- Use it as the starting point for relative path resolution

## 4. FileSystem Trait Implementation

### 4.1 canonicalize
- Convert a path to its canonical form
- Resolve and normalize the path

### 4.2 write
- Create or update a file with given contents
- Create necessary parent directories if they don't exist

### 4.3 read_to_string
- Read file contents as a string
- Return an error if the file doesn't exist or is a directory

### 4.4 mkdir_p
- Create directories recursively
- No error if the directory already exists

### 4.5 remove_file
- Delete a file
- Return an error if the path doesn't exist or is a directory

### 4.6 remove_dir
- Delete an empty directory
- Return an error if the path doesn't exist, is not a directory, or the directory is not empty

## 5. Error Handling
- Implement proper error handling for all operations
- Mimic real file system error behavior (e.g., "No such file or directory", "Permission denied")
- Use io::Error and appropriate ErrorKind variants

## 6. Thread Safety
- Ensure thread-safety for potential use in multi-threaded tests
- Consider using interior mutability pattern (e.g., RwLock) for the file system structure

## 7. Initialization and Testing
- Implement a method to initialize the VirtualFileSystem with a predefined structure
- Provide helper methods for setting up test scenarios

## 8. Special Considerations
- Handle the root directory ('/') as a special case
- Ensure that attempts to navigate above the root (e.g., '/../') are handled correctly
- Implement proper handling of relative paths in all operations
