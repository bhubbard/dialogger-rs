use dialogger_rs::{Bark, BarkCategory, BarkManager, VariableStore};

#[test]
fn test_bark_priority_and_category_selection() {
    let mut manager = BarkManager::new();
    manager.seed(42);

    manager.add_bark(Bark {
        id: "low_alert".into(),
        speaker: "Guard".into(),
        category: BarkCategory::CombatAlert,
        text: "Did I hear a rat?".into(),
        voice_asset: None,
        priority: 5,
        weight: 1.0,
        cooldown: 10.0,
        condition: None,
    });

    manager.add_bark(Bark {
        id: "high_alert".into(),
        speaker: "Guard".into(),
        category: BarkCategory::CombatAlert,
        text: "Intruder! Sound the alarm!".into(),
        voice_asset: None,
        priority: 20,
        weight: 1.0,
        cooldown: 10.0,
        condition: None,
    });

    let vars = VariableStore::new();

    // Priority 20 should be selected over priority 5
    let chosen = manager.trigger_bark(&BarkCategory::CombatAlert, &vars, 0.0);
    assert!(chosen.is_some());
    let bark = chosen.unwrap();
    assert_eq!(bark.id, "high_alert");
    assert_eq!(bark.text, "Intruder! Sound the alarm!");

    // Immediate next trigger at t=1.0: "high_alert" is on 10s cooldown, so "low_alert" (priority 5) triggers
    let second = manager.trigger_bark(&BarkCategory::CombatAlert, &vars, 1.0);
    assert!(second.is_some());
    assert_eq!(second.unwrap().id, "low_alert");

    // At t=2.0, both are on cooldown
    let third = manager.trigger_bark(&BarkCategory::CombatAlert, &vars, 2.0);
    assert!(third.is_none());

    // At t=11.0, high_alert has cooled down
    let fourth = manager.trigger_bark(&BarkCategory::CombatAlert, &vars, 11.0);
    assert!(fourth.is_some());
    assert_eq!(fourth.unwrap().id, "high_alert");
}

#[test]
fn test_bark_condition_filtering() {
    let mut manager = BarkManager::new();

    manager.add_bark(Bark {
        id: "low_hp_bark".into(),
        speaker: "Bandit".into(),
        category: BarkCategory::Fleeing,
        text: "I'm bleeding out! Retreat!".into(),
        voice_asset: None,
        priority: 10,
        weight: 1.0,
        cooldown: 5.0,
        condition: Some("health < 25".into()),
    });

    let mut vars = VariableStore::new();
    vars.set("health", 100);

    // Should not trigger when health = 100
    assert!(manager.trigger_bark(&BarkCategory::Fleeing, &vars, 0.0).is_none());

    // Should trigger when health = 15
    vars.set("health", 15);
    let chosen = manager.trigger_bark(&BarkCategory::Fleeing, &vars, 0.0);
    assert!(chosen.is_some());
    assert_eq!(chosen.unwrap().id, "low_hp_bark");
}
