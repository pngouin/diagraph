use std::path::Path;

use diagraph::{discover, validate};

#[test]
fn example_monorepo_is_valid_and_has_seven_components() {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/monorepo"));
    let graph = discover::scan(root).unwrap();
    assert_eq!(graph.components.len(), 7);
    assert!(validate::validate(&graph).is_empty());
}

#[test]
fn report_generator_declares_the_expected_parts_and_attribution() {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/monorepo"));
    let graph = discover::scan(root).unwrap();

    let report_generator = graph.find("report-generator").unwrap();
    assert_eq!(report_generator.parts.len(), 2);
    let fetch_thread = report_generator.find_part("fetch-thread").unwrap();
    assert_eq!(fetch_thread.edges[0].target, "upload-thread");
    let bucket_edge = &report_generator.edges[0];
    assert_eq!(bucket_edge.from_part.as_deref(), Some("upload-thread"));

    let api_gateway = graph.find("api-gateway").unwrap();
    let to_report_generator = api_gateway
        .edges
        .iter()
        .find(|e| e.target.name() == "report-generator")
        .unwrap();
    assert_eq!(to_report_generator.to_part.as_deref(), Some("fetch-thread"));
}
