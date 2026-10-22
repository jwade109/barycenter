mod file;
mod folder;
mod node;

#[cfg(test)]
mod tests;

pub mod prelude;

use super::prelude::*;

use crate::World;

pub struct Filesystem {
    /// folder at /
    root: Folder,
}

pub enum FileSystemError {
    NotADirectory,
    NoSuchFile,
}

impl Filesystem {
    pub fn new() -> Self {
        Self {
            root: Folder::new(),
        }
    }

    pub fn from_root(root: Folder) -> Self {
        Self { root }
    }

    pub fn tree(&self) -> String {
        self.root.tree()
    }
}

pub fn virtual_filesystem(world: &World) -> Filesystem {
    let mut ships = Folder::new();

    for grid in world.grids.values() {
        let parts = Folder::new();
        ships.insert(grid.name.clone(), parts);
    }

    let mut root = Folder::new();
    root.insert("ships", ships);

    let mut planets = Folder::new();

    for (id, planet) in world.planets.iter() {
        planets.insert(format!("{}", id), File {});
    }

    root.insert("planets", planets);

    Filesystem::from_root(root)
}
