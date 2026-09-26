use dialogger_rs::{
    eval_expression, ChoiceOption, DialogueRunner, DialogueStep, GraphBuilder, MutationOp, Value,
    VariableStore,
};

#[test]
fn test_expression_evaluator() {
    let mut vars = VariableStore::new();
    vars.set("gold", 150);
    vars.set("has_amulet", true);
    vars.set("reputation", 4.5);
    vars.set("faction", "guild");

    assert!(eval_expression("gold >= 100", &vars).unwrap());
    assert!(!eval_expression("gold < 50", &vars).unwrap());
    assert!(eval_expression("has_amulet && gold > 100", &vars).unwrap());
    assert!(eval_expression("!has_amulet || gold == 150", &vars).unwrap());
    assert!(eval_expression("faction == 'guild'", &vars).unwrap());
    assert!(eval_expression("reputation > 4.0 && (gold == 150 || gold == 0)", &vars).unwrap());
    assert!(eval_expression("not (gold < 100)", &vars).unwrap());
}

#[test]
fn test_dialogue_traversal_with_choice_and_variable_mutation() {
    let graph = GraphBuilder::new()
        .speech("intro", "Merchant", "Got any gold on ya?", Some("ask_choice"))
        .choice(
            "ask_choice",
            Some("How do you respond?"),
            vec![
                ChoiceOption {
                    id: "pay_tax".into(),
                    text: "Here is 50 gold.".into(),
                    target: "deduct_gold".into(),
                    condition: Some("gold >= 50".into()),
                },
                ChoiceOption {
                    id: "bribe_expensive".into(),
                    text: "Take 200 gold and keep quiet.".into(),
                    target: "deduct_200".into(),
                    condition: Some("gold >= 200".into()),
                },
                ChoiceOption {
                    id: "refuse".into(),
                    text: "Not a copper!".into(),
                    target: "angry_merchant".into(),
                    condition: None,
                },
            ],
        )
        .set_variable("deduct_gold", "gold", MutationOp::Subtract, Value::Int(50), Some("thank_you"))
        .speech("thank_you", "Merchant", "A pleasure doing business.", None)
        .speech("angry_merchant", "Merchant", "Then be gone with you!", None)
        .speech("deduct_200", "Merchant", "Very generous!", None)
        .build()
        .unwrap();

    let mut runner = DialogueRunner::new(&graph);
    runner.variables.set("gold", 100);

    // Step 1: intro speech
    match runner.step().unwrap() {
        DialogueStep::Speech { speech, .. } => {
            assert_eq!(speech.speaker, "Merchant");
            assert_eq!(speech.text, "Got any gold on ya?");
        }
        other => panic!("Expected Speech step, got {:?}", other),
    }

    // Step 2: choice
    match runner.step().unwrap() {
        DialogueStep::Choice { options, .. } => {
            assert_eq!(options.len(), 3);
            assert!(options[0].available); // 50 gold <= 100
            assert!(!options[1].available); // 200 gold > 100 (locked)
            assert!(options[2].available); // no condition
        }
        other => panic!("Expected Choice step, got {:?}", other),
    }

    // Select choice: pay_tax
    runner.select_choice_by_id("pay_tax").unwrap();

    // Step 3: set_variable is processed internally, moves to thank_you speech
    match runner.step().unwrap() {
        DialogueStep::Speech { speech, .. } => {
            assert_eq!(speech.speaker, "Merchant");
            assert_eq!(speech.text, "A pleasure doing business.");
        }
        other => panic!("Expected Speech step, got {:?}", other),
    }

    // Check variable state after mutation: 100 - 50 = 50
    assert_eq!(runner.variables.get("gold"), Some(&Value::Int(50)));

    // Step 4: end
    assert_eq!(runner.step().unwrap(), DialogueStep::End);

    // History verification
    assert_eq!(
        runner.history(),
        &["intro", "ask_choice", "deduct_gold", "thank_you"]
    );
}

#[test]
fn test_condition_node_branching() {
    let graph = GraphBuilder::new()
        .condition("check_key", "has_key == true", Some("open_door"), Some("locked_door"))
        .speech("open_door", "Narrator", "The door creaks open.", None)
        .speech("locked_door", "Narrator", "The door is locked shut.", None)
        .build()
        .unwrap();

    // Test false branch
    let mut runner1 = DialogueRunner::new(&graph);
    runner1.variables.set("has_key", false);
    match runner1.step().unwrap() {
        DialogueStep::Speech { speech, .. } => assert_eq!(speech.text, "The door is locked shut."),
        other => panic!("Unexpected: {:?}", other),
    }

    // Test true branch
    let mut runner2 = DialogueRunner::new(&graph);
    runner2.variables.set("has_key", true);
    match runner2.step().unwrap() {
        DialogueStep::Speech { speech, .. } => assert_eq!(speech.text, "The door creaks open."),
        other => panic!("Unexpected: {:?}", other),
    }
}
