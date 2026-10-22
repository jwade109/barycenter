use std::collections::BTreeMap;

use super::prelude::*;

#[derive(Debug, Clone)]
pub struct Folder {
    files: BTreeMap<String, Node>,
}

impl Folder {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Node)> {
        self.files.iter()
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Node>) {
        self.files.insert(key.into(), value.into());
    }

    fn tree_inner(&self, offset: usize) -> Vec<String> {
        let mut ret = Vec::new();
        for (name, node) in &self.files {
            ret.push(name.clone());

            match node {
                Node::Folder(folder) => {
                    for line in folder.tree_inner(offset + 2) {
                        ret.push(format!("  {}", line));
                    }
                }
                Node::RegularFile(_) => (),
            }
        }
        ret
    }

    pub fn tree(&self) -> String {
        self.tree_inner(0).join("\n")
    }
}
