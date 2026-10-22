use super::prelude::*;

#[test]
fn filesystem_tree_repr() {
    let mut subsubfolder = Folder::new();
    subsubfolder.insert("x", File {});
    subsubfolder.insert("y", File {});
    subsubfolder.insert("z", File {});

    let mut subfolder = Folder::new();
    subfolder.insert("inner_a", File {});
    subfolder.insert("inner_b", File {});
    subfolder.insert("inner_c", subsubfolder);
    subfolder.insert("inner_d", File {});

    let mut folder = Folder::new();

    folder.insert("file_a", File {});
    folder.insert("file_b", File {});
    folder.insert("file_c", File {});
    folder.insert("dir_a", subfolder);

    let fs = Filesystem::from_root(folder);

    println!("{}", fs.tree());
}
