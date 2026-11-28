#![allow(clippy::unwrap_used, reason = "fixture failures should panic")]

use super::{Env, resolve_admiral_room};
use formation_store::Store;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, directory: &Path, entries: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                entries.insert(relative, None);
                visit(root, &path, entries);
            } else {
                entries.insert(relative, Some(std::fs::read(path).unwrap()));
            }
        }
    }

    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

#[test]
fn resolving_one_active_room_reads_without_changing_home_files() {
    let root = cw_testkit::TempRoot::new("FORMATION").unwrap();
    let home = root.path().join("formation-home");
    let room_path = home.join("rooms/trial");
    Store::create(&room_path)
        .unwrap()
        .write(|_| Ok(()))
        .unwrap();

    let env = Env {
        home: Some(home.clone().into_os_string()),
        ..Env::default()
    };
    let before = snapshot(&home);
    let room = resolve_admiral_room(&env).unwrap();

    assert_eq!(room.record, "trial");
    assert_eq!(snapshot(&home), before);
}
