//! Check the STD-12 rules that stable rustfmt and Clippy do not enforce.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{File, Item, ItemImpl, ItemMod, ItemTrait, ItemUse, UseTree};

struct Source<'a> {
    lines: Vec<&'a str>,
    path: &'a Path,
    failures: &'a mut Vec<String>,
}

impl Source<'_> {
    fn report(&mut self, line: usize, message: &str) {
        self.failures
            .push(format!("{}:{line}: {message}", self.path.display()));
    }

    fn separate(&mut self, previous: usize, next: usize) {
        if previous > 0
            && next > previous
            && !self.lines[previous..next - 1]
                .iter()
                .any(|line| line.trim().is_empty())
        {
            self.report(next, "missing blank line before item");
        }
    }

    fn items(&mut self, mut previous: usize, items: &[Item]) {
        let mut was_import = false;

        for item in items {
            let import = matches!(item, Item::Use(_));
            if !(import && was_import) {
                self.separate(previous, item.span().start().line);
            }
            previous = item.span().end().line;
            was_import = import;
        }
    }

    fn members(&mut self, members: &[impl Spanned]) {
        for pair in members.windows(2) {
            self.separate(pair[0].span().end().line, pair[1].span().start().line);
        }
    }

    fn lines(&mut self) {
        for index in 0..self.lines.len() {
            let line = self.lines[index];
            let trimmed = line.trim().trim_end_matches(',');
            if line.chars().count() > 100
                && syn::parse_str::<syn::LitStr>(trimmed).is_err()
                && !trimmed
                    .strip_prefix("/// ")
                    .unwrap_or(trimmed)
                    .starts_with("https://")
            {
                self.report(index + 1, "line wider than 100 columns");
            }
            if trimmed == "#[rustfmt::skip]" {
                let reason = index
                    .checked_sub(1)
                    .map_or("", |offset| self.lines[offset].trim());
                if !reason.starts_with("//")
                    || !(reason.contains("align") || reason.contains("generat"))
                {
                    self.report(
                        index + 1,
                        "rustfmt skip needs an alignment or generation comment",
                    );
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for Source<'_> {
    fn visit_file(&mut self, file: &'ast File) {
        let header = file.attrs.last().map_or(0, |attr| attr.span().end().line);
        self.items(header, &file.items);
        visit::visit_file(self, file);
    }

    fn visit_item_mod(&mut self, module: &'ast ItemMod) {
        if let Some((_, items)) = &module.content {
            self.items(0, items);
        }
        visit::visit_item_mod(self, module);
    }

    fn visit_item_impl(&mut self, block: &'ast ItemImpl) {
        self.members(&block.items);
        visit::visit_item_impl(self, block);
    }

    fn visit_item_trait(&mut self, block: &'ast ItemTrait) {
        self.members(&block.items);
        visit::visit_item_trait(self, block);
    }

    fn visit_item_use(&mut self, import: &'ast ItemUse) {
        let mut tree = &import.tree;
        while let UseTree::Path(path) = tree {
            tree = &path.tree;
        }
        if let UseTree::Group(group) = tree
            && group
                .items
                .iter()
                .any(|item| matches!(item, UseTree::Path(_) | UseTree::Group(_)))
        {
            self.report(import.span().start().line, "import from one module per use");
        }
    }
}

fn sources(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            sources(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn plural(name: &str) -> bool {
    name.ends_with('s')
        && !matches!(
            name,
            "tests" | "benches" | "examples" | "status" | "address"
        )
}

#[test]
fn workspace_source_follows_std_12() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()?;
    let mut files = Vec::new();
    sources(&root, &mut files)?;
    files.sort();

    let mut failures = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path)?;
        if source
            .lines()
            .take(3)
            .any(|line| line.trim_start().starts_with("// @generated"))
        {
            continue;
        }

        let relative = path.strip_prefix(&root)?;
        for name in relative.with_extension("").iter().skip(1) {
            if plural(&name.to_string_lossy()) {
                failures.push(format!("{}: plural source name {:?}", path.display(), name));
            }
        }

        let mut check = Source {
            path: &path,
            lines: source.lines().collect(),
            failures: &mut failures,
        };
        check.lines();
        check.visit_file(&syn::parse_file(&source)?);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

#[test]
fn grouped_imports_and_adjacent_methods_are_reported() -> Result<(), Box<dyn Error>> {
    let fixture = concat!(
        "//! Header\n",
        "use std::{fmt, io::Read};\n\n",
        "struct Seat;\n\n",
        "impl Seat {\n",
        "    fn first() {}\n",
        "    fn second() {}\n",
        "}\n",
    );
    let mut failures = Vec::new();
    let mut check = Source {
        path: Path::new("fixture.rs"),
        lines: fixture.lines().collect(),
        failures: &mut failures,
    };
    check.visit_file(&syn::parse_file(fixture)?);

    assert_eq!(failures.len(), 3);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("fixture.rs:8: missing blank"))
    );
    Ok(())
}
