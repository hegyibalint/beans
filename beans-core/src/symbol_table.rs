use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::{Symbol, SymbolId, SymbolKind};

pub struct SymbolTable {
    arena: Vec<Symbol>,
    removed: HashSet<SymbolId>,
    fqn_index: HashMap<String, SymbolId>,
    package_index: HashMap<String, Vec<SymbolId>>,
    file_index: HashMap<PathBuf, Vec<SymbolId>>,
    kind_index: HashMap<SymbolKind, Vec<SymbolId>>,
    name_index: HashMap<String, Vec<SymbolId>>,
    parent_index: HashMap<SymbolId, Vec<SymbolId>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            arena: Vec::new(),
            removed: HashSet::new(),
            fqn_index: HashMap::new(),
            package_index: HashMap::new(),
            file_index: HashMap::new(),
            kind_index: HashMap::new(),
            name_index: HashMap::new(),
            parent_index: HashMap::new(),
        }
    }

    pub fn insert(&mut self, mut symbol: Symbol) -> SymbolId {
        let id = SymbolId(self.arena.len());
        symbol.id = id;

        // Update indexes
        self.fqn_index.insert(symbol.fqn.clone(), id);

        // Package index: derive package from FQN (everything before last dot)
        if let Some(dot_pos) = symbol.fqn.rfind('.') {
            let package = symbol.fqn[..dot_pos].to_string();
            self.package_index.entry(package).or_default().push(id);
        }

        if let Some(ref loc) = symbol.location {
            self.file_index
                .entry(loc.file.clone())
                .or_default()
                .push(id);
        }

        self.kind_index.entry(symbol.kind).or_default().push(id);
        self.name_index
            .entry(symbol.name.clone())
            .or_default()
            .push(id);

        if let Some(parent) = symbol.parent {
            self.parent_index.entry(parent).or_default().push(id);
        }

        self.arena.push(symbol);
        id
    }

    /// Returns the total number of symbols ever inserted (including removed ones).
    /// Useful for computing the next available index offset.
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    pub fn is_empty(&self) -> bool {
        self.arena.len() == self.removed.len()
    }

    pub fn get(&self, id: SymbolId) -> Option<&Symbol> {
        if self.removed.contains(&id) {
            return None;
        }
        self.arena.get(id.0)
    }

    pub fn get_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        if self.removed.contains(&id) {
            return None;
        }
        self.arena.get_mut(id.0)
    }

    pub fn lookup_by_fqn(&self, fqn: &str) -> Option<SymbolId> {
        self.fqn_index.get(fqn).copied().filter(|id| !self.removed.contains(id))
    }

    pub fn lookup_by_name(&self, name: &str) -> Vec<SymbolId> {
        self.name_index
            .get(name)
            .map(|ids| ids.iter().copied().filter(|id| !self.removed.contains(id)).collect())
            .unwrap_or_default()
    }

    pub fn lookup_by_package(&self, package: &str) -> Vec<SymbolId> {
        self.package_index
            .get(package)
            .map(|ids| ids.iter().copied().filter(|id| !self.removed.contains(id)).collect())
            .unwrap_or_default()
    }

    pub fn lookup_children(&self, parent: SymbolId) -> Vec<SymbolId> {
        self.parent_index
            .get(&parent)
            .map(|ids| ids.iter().copied().filter(|id| !self.removed.contains(id)).collect())
            .unwrap_or_default()
    }

    pub fn lookup_by_kind(&self, kind: SymbolKind) -> Vec<SymbolId> {
        self.kind_index
            .get(&kind)
            .map(|ids| ids.iter().copied().filter(|id| !self.removed.contains(id)).collect())
            .unwrap_or_default()
    }

    pub fn lookup_by_file(&self, file: &Path) -> Vec<SymbolId> {
        self.file_index
            .get(file)
            .map(|ids| ids.iter().copied().filter(|id| !self.removed.contains(id)).collect())
            .unwrap_or_default()
    }

    /// Insert symbols from a parser, remapping local parent indices to real SymbolIds.
    /// Parsers produce symbols with local indices (0, 1, 2...) for parent references.
    /// This method assigns real SymbolIds and fixes up parent/children relationships.
    pub fn insert_parsed_symbols(&mut self, symbols: Vec<Symbol>) {
        let mut id_map: Vec<SymbolId> = Vec::with_capacity(symbols.len());

        for mut symbol in symbols {
            let local_parent = symbol.parent;
            let _local_children = std::mem::take(&mut symbol.children);
            symbol.parent = local_parent.and_then(|p| id_map.get(p.0).copied());
            symbol.children = vec![];

            let real_id = self.insert(symbol);
            id_map.push(real_id);

            if let Some(local_parent_idx) = local_parent {
                if let Some(&real_parent_id) = id_map.get(local_parent_idx.0) {
                    if let Some(parent_sym) = self.get_mut(real_parent_id) {
                        parent_sym.children.push(real_id);
                    }
                }
            }
        }
    }

    pub fn remove_by_file(&mut self, file: &Path) {
        let ids = match self.file_index.remove(file) {
            Some(ids) => ids,
            None => return,
        };

        for id in &ids {
            self.removed.insert(*id);

            if let Some(symbol) = self.arena.get(id.0) {
                self.fqn_index.remove(&symbol.fqn);

                if let Some(dot_pos) = symbol.fqn.rfind('.') {
                    let package = &symbol.fqn[..dot_pos];
                    if let Some(pkg_ids) = self.package_index.get_mut(package) {
                        pkg_ids.retain(|i| i != id);
                    }
                }

                if let Some(kind_ids) = self.kind_index.get_mut(&symbol.kind) {
                    kind_ids.retain(|i| i != id);
                }

                if let Some(name_ids) = self.name_index.get_mut(&symbol.name) {
                    name_ids.retain(|i| i != id);
                }

                if let Some(parent) = symbol.parent {
                    if let Some(parent_ids) = self.parent_index.get_mut(&parent) {
                        parent_ids.retain(|i| i != id);
                    }
                }
            }
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Location, Modifier};
    use std::path::PathBuf;

    fn make_class(fqn: &str, name: &str, file: &str, parent: Option<SymbolId>) -> Symbol {
        Symbol {
            id: SymbolId(0), // will be overwritten by insert
            fqn: fqn.to_string(),
            name: name.to_string(),
            kind: SymbolKind::Class,
            location: Some(Location {
                file: PathBuf::from(file),
                start_line: 1,
                start_col: 0,
                end_line: 10,
                end_col: 0,
            }),
            modifiers: vec![Modifier::Public],
            annotations: vec![],
            parent,
            children: vec![],
            relations: vec![],
            signature: None,
        }
    }

    fn make_method(
        fqn: &str,
        name: &str,
        file: &str,
        parent: Option<SymbolId>,
    ) -> Symbol {
        Symbol {
            id: SymbolId(0),
            fqn: fqn.to_string(),
            name: name.to_string(),
            kind: SymbolKind::Method,
            location: Some(Location {
                file: PathBuf::from(file),
                start_line: 5,
                start_col: 4,
                end_line: 8,
                end_col: 4,
            }),
            modifiers: vec![Modifier::Public],
            annotations: vec![],
            parent,
            children: vec![],
            relations: vec![],
            signature: None,
        }
    }

    #[test]
    fn insert_and_lookup_by_fqn() {
        let mut table = SymbolTable::new();
        let id = table.insert(make_class(
            "com.example.MyClass",
            "MyClass",
            "src/MyClass.java",
            None,
        ));
        assert_eq!(table.lookup_by_fqn("com.example.MyClass"), Some(id));
        assert_eq!(table.lookup_by_fqn("com.example.Other"), None);
    }

    #[test]
    fn lookup_by_name() {
        let mut table = SymbolTable::new();
        let id = table.insert(make_class(
            "com.example.MyClass",
            "MyClass",
            "src/MyClass.java",
            None,
        ));
        let results = table.lookup_by_name("MyClass");
        assert_eq!(results, vec![id]);
        assert!(table.lookup_by_name("Other").is_empty());
    }

    #[test]
    fn lookup_by_package() {
        let mut table = SymbolTable::new();
        let id = table.insert(make_class(
            "com.example.MyClass",
            "MyClass",
            "src/MyClass.java",
            None,
        ));
        let results = table.lookup_by_package("com.example");
        assert_eq!(results, vec![id]);
        assert!(table.lookup_by_package("com.other").is_empty());
    }

    #[test]
    fn lookup_by_kind() {
        let mut table = SymbolTable::new();
        let class_id = table.insert(make_class(
            "com.example.MyClass",
            "MyClass",
            "src/MyClass.java",
            None,
        ));
        let method_id = table.insert(make_method(
            "com.example.MyClass.doWork",
            "doWork",
            "src/MyClass.java",
            Some(class_id),
        ));

        let classes = table.lookup_by_kind(SymbolKind::Class);
        assert_eq!(classes, vec![class_id]);

        let methods = table.lookup_by_kind(SymbolKind::Method);
        assert_eq!(methods, vec![method_id]);
    }

    #[test]
    fn lookup_by_file() {
        let mut table = SymbolTable::new();
        let id1 = table.insert(make_class(
            "com.example.MyClass",
            "MyClass",
            "src/MyClass.java",
            None,
        ));
        let id2 = table.insert(make_method(
            "com.example.MyClass.doWork",
            "doWork",
            "src/MyClass.java",
            Some(id1),
        ));
        let _id3 = table.insert(make_class(
            "com.example.Other",
            "Other",
            "src/Other.java",
            None,
        ));

        let results = table.lookup_by_file(Path::new("src/MyClass.java"));
        assert_eq!(results, vec![id1, id2]);
    }

    #[test]
    fn parent_child_relationships() {
        let mut table = SymbolTable::new();
        let class_id = table.insert(make_class(
            "com.example.MyClass",
            "MyClass",
            "src/MyClass.java",
            None,
        ));
        let method_id = table.insert(make_method(
            "com.example.MyClass.doWork",
            "doWork",
            "src/MyClass.java",
            Some(class_id),
        ));

        let children = table.lookup_children(class_id);
        assert_eq!(children, vec![method_id]);

        let symbol = table.get(method_id).unwrap();
        assert_eq!(symbol.parent, Some(class_id));
    }

    #[test]
    fn remove_by_file_clears_all_indexes() {
        let mut table = SymbolTable::new();
        let class_id = table.insert(make_class(
            "com.example.MyClass",
            "MyClass",
            "src/MyClass.java",
            None,
        ));
        let _method_id = table.insert(make_method(
            "com.example.MyClass.doWork",
            "doWork",
            "src/MyClass.java",
            Some(class_id),
        ));

        // Other file should be unaffected
        let other_id = table.insert(make_class(
            "com.example.Other",
            "Other",
            "src/Other.java",
            None,
        ));

        table.remove_by_file(Path::new("src/MyClass.java"));

        // All lookups for removed symbols should return empty/None
        assert_eq!(table.lookup_by_fqn("com.example.MyClass"), None);
        assert_eq!(table.lookup_by_fqn("com.example.MyClass.doWork"), None);
        assert!(table.lookup_by_name("MyClass").is_empty());
        assert!(table.lookup_by_name("doWork").is_empty());
        assert!(table.lookup_by_kind(SymbolKind::Method).is_empty());
        assert!(table.lookup_by_file(Path::new("src/MyClass.java")).is_empty());
        assert!(table.lookup_children(class_id).is_empty());
        assert!(table.get(class_id).is_none());

        // Other file untouched
        assert_eq!(table.lookup_by_fqn("com.example.Other"), Some(other_id));
        assert!(table.get(other_id).is_some());
    }

    #[test]
    fn reinsert_after_removal() {
        let mut table = SymbolTable::new();
        let _old_id = table.insert(make_class(
            "com.example.MyClass",
            "MyClass",
            "src/MyClass.java",
            None,
        ));

        table.remove_by_file(Path::new("src/MyClass.java"));

        // Re-insert the same class
        let new_id = table.insert(make_class(
            "com.example.MyClass",
            "MyClass",
            "src/MyClass.java",
            None,
        ));

        assert_eq!(table.lookup_by_fqn("com.example.MyClass"), Some(new_id));
        assert!(table.get(new_id).is_some());
        assert_eq!(table.lookup_by_name("MyClass"), vec![new_id]);
        assert_eq!(
            table.lookup_by_file(Path::new("src/MyClass.java")),
            vec![new_id]
        );
    }
}
