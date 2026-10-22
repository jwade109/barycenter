use super::prelude::*;

#[derive(Debug, Clone)]
pub enum Node {
    RegularFile(File),
    Folder(Folder),
}

impl Node {
    pub fn tree(&self) -> String {
        match self {
            Self::RegularFile(_) => String::new(),
            Self::Folder(folder) => folder.tree(),
        }
    }
}

impl From<Folder> for Node {
    fn from(value: Folder) -> Self {
        Self::Folder(value)
    }
}

impl From<File> for Node {
    fn from(value: File) -> Self {
        Self::RegularFile(value)
    }
}
