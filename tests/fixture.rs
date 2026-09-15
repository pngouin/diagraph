use std::path::Path;

use diagraph::{discover, validate};

#[test]
fn example_monorepo_is_valid_and_has_seven_components() {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/monorepo"));
    let graph = discover::scan(root).unwrap();
    assert_eq!(graph.components.len(), 7);
    assert!(validate::validate(&graph).is_empty());
}
