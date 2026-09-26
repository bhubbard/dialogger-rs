use dialogger_rs::{DialogueGraph, DialogueNode, GraphBuilder, GraphError, MutationOp, SpeechNode, Value};

#[test]
fn test_graph_builder_and_validation() {
    let graph = GraphBuilder::new()
        .title("Tavern Encounter")
        .speech("node_1", "Innkeeper", "Welcome to the Prancing Pony!", Some("node_2"))
        .speech("node_2", "Player", "Thank you, what's on tap?", None)
        .build()
        .expect("Valid graph should build successfully");

    assert_eq!(graph.title, "Tavern Encounter");
    assert_eq!(graph.start_node(), "node_1");
    assert_eq!(graph.nodes.len(), 2);
}

#[test]
fn test_graph_broken_link_detection() {
    let err = GraphBuilder::new()
        .speech("start", "Guide", "Follow me...", Some("ghost_node"))
        .build()
        .expect_err("Should detect broken link");

    match err {
        GraphError::BrokenLink { from, to } => {
            assert_eq!(from, "start");
            assert_eq!(to, "ghost_node");
        }
        other => panic!("Unexpected error: {:?}", other),
    }
}

#[test]
fn test_graph_orphan_detection() {
    let mut graph = DialogueGraph::new("start");
    graph.add_node(
        "start",
        DialogueNode::Speech(SpeechNode {
            speaker: "Narrator".into(),
            text: "Beginning".into(),
            voice_asset: None,
            expression: None,
            next: None,
        }),
    );
    graph.add_node(
        "orphan_node",
        DialogueNode::Speech(SpeechNode {
            speaker: "Nobody".into(),
            text: "I am lost in the ether".into(),
            voice_asset: None,
            expression: None,
            next: None,
        }),
    );

    let orphans = graph.validate().expect("Validation succeeds with warnings");
    assert!(orphans.contains("orphan_node"));
    assert_eq!(orphans.len(), 1);
}

#[test]
fn test_json_roundtrip() {
    let original = GraphBuilder::new()
        .title("Quest Initiation")
        .speech("start", "Elder", "A dragon awakens in the mountains.", Some("set_flag"))
        .set_variable("set_flag", "quest_started", MutationOp::Assign, Value::Bool(true), Some("end"))
        .speech("end", "Elder", "May fortune guide your blade.", None)
        .build()
        .unwrap();

    let json_str = original.to_json().expect("Should serialize to JSON");
    let parsed = DialogueGraph::from_json(&json_str).expect("Should deserialize from JSON");

    assert_eq!(original.title, parsed.title);
    assert_eq!(original.start_node, parsed.start_node);
    assert_eq!(original.nodes.len(), parsed.nodes.len());
}

#[test]
fn test_classic_dialogger_json_import() {
    let dialogger_json = r#"[
        {
            "id": 1,
            "character": "Garrison Guard",
            "text": "Halt! Who goes there?",
            "choices": [
                {
                    "text": "I am a humble merchant.",
                    "node": 2
                },
                {
                    "text": "Out of my way or draw steel!",
                    "node": 3
                }
            ]
        },
        {
            "id": 2,
            "character": "Garrison Guard",
            "text": "Very well, pay the toll and enter.",
            "next": null
        },
        {
            "id": 3,
            "character": "Garrison Guard",
            "text": "To arms! We are under attack!",
            "next": null
        }
    ]"#;

    let graph = DialogueGraph::from_dialogger_json(dialogger_json).expect("Should parse classic Dialogger JSON");
    assert_eq!(graph.start_node, "1");
    assert!(graph.get_node("1").is_some());
    assert!(graph.get_node("2").is_some());
    assert!(graph.get_node("3").is_some());
}
