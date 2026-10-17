//! RXP guide importer and replay verifier (task V2.3). Synthetic guides and facts
//! only: every quest, zone and node here is invented.
use azeroth_ingest::{
    rxp::{
        self, Applies, Character, Class, Directive, Edition, HeaderTag, Item, NoteKind, Race,
        Rejection, StepTag, StructureIssue, XpKind,
    },
    rxp_verify::{
        self, Allowed, Check, Fact, FactOrigin, FactSource, FactsRejection, JsonFacts, QuestFacts,
        QuestRecord, Report, Status,
    },
};
use std::collections::{BTreeMap, BTreeSet};

const HUMAN_WARRIOR: Character = Character {
    race: Race::Human,
    class: Class::Warrior,
};

// ---------------------------------------------------------------------------
// In-test fact source
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Mem {
    quests: BTreeMap<u32, QuestFacts>,
    absent: BTreeSet<u32>,
    zones: Option<BTreeSet<&'static str>>,
    nodes: Option<BTreeSet<&'static str>>,
    capacity: Option<u16>,
    spells: BTreeMap<u32, BTreeSet<Class>>,
    level_xp: Option<Vec<u32>>,
}
impl FactSource for Mem {
    fn quest(&self, id: u32) -> QuestRecord {
        match self.quests.get(&id) {
            Some(q) => QuestRecord::Known(q.clone()),
            None if self.absent.contains(&id) => QuestRecord::Absent,
            None => QuestRecord::Unknown,
        }
    }
    fn zone(&self, name: &str) -> Fact<bool> {
        self.zones.as_ref().map(|z| z.contains(name)).into()
    }
    fn flight_node(&self, name: &str) -> Fact<bool> {
        self.nodes.as_ref().map(|z| z.contains(name)).into()
    }
    fn quest_log_capacity(&self) -> Fact<u16> {
        self.capacity.into()
    }
    fn spell_trainable(&self, spell: u32, class: Class) -> Fact<bool> {
        self.spells.get(&spell).map(|c| c.contains(&class)).into()
    }
    fn level_xp(&self, level: u8) -> Fact<u32> {
        self.level_xp
            .as_ref()
            .and_then(|t| t.get(usize::from(level) - 1).copied())
            .into()
    }
}

/// A fully known, unrestricted quest with `objectives` objectives.
fn quest(objectives: u8) -> QuestFacts {
    QuestFacts {
        required_level: Fact::Known(1),
        level: Fact::Known(1),
        prerequisites_any: Fact::Known(vec![]),
        prerequisites_all: Fact::Known(vec![]),
        races: Fact::Known(Allowed::Any),
        classes: Fact::Known(Allowed::Any),
        repeatable: Fact::Known(false),
        seasonal: Fact::Known(false),
        objectives: Fact::Known(objectives),
        reward_choices: Fact::Known(0),
        xp: Fact::Known(10),
    }
}

/// Facts that know every place used below and quests 1..=9 with no objectives.
fn world() -> Mem {
    Mem {
        quests: (1..=9).map(|q| (q, quest(0))).collect(),
        absent: [404].into(),
        zones: Some(["Test Vale", "Sample Hills"].into()),
        nodes: Some(["Testford"].into()),
        capacity: Some(20),
        spells: [(100, [Class::Warrior].into())].into(),
        level_xp: Some(vec![400; 59]),
    }
}

fn guide(steps: &str) -> String {
    format!(
        "RXPGuides.RegisterGuide([[\n#forever\n<< Human\n#name 1-5 Test Vale\n#version 1\n#group Synthetic\n\n{steps}\n]])\n"
    )
}
fn run(text: &str, facts: &Mem) -> Report {
    let file = rxp::parse(text.as_bytes()).expect("synthetic guide parses");
    rxp_verify::verify(&file, facts, HUMAN_WARRIOR, Some(FactOrigin::Synthetic))
}
fn findings(report: &Report) -> Vec<(Check, Status)> {
    report
        .file_findings
        .iter()
        .chain(report.guides.iter().flat_map(|g| &g.findings))
        .chain(report.steps.iter().flat_map(|s| &s.findings))
        .map(|f| (f.check, f.status))
        .collect()
}
fn has(report: &Report, check: Check, status: Status) -> bool {
    findings(report).contains(&(check, status))
}
#[track_caller]
fn clean(report: &Report) {
    assert!(
        report.passes && report.totals.warnings == 0,
        "unexpected findings: {:?}",
        findings(report)
    );
}

// ---------------------------------------------------------------------------
// Importer
// ---------------------------------------------------------------------------

#[test]
fn parses_header_tags_and_filters() {
    let text = "-- leading comment\n\nRXPGuides.RegisterGuide([[\n#forever\n<< Human Warrior\n#name 1-6 Test Vale\n#displayname 1-6 Vale\n#version 2\n#group Synthetic\n#subgroup Part\n#defaultfor Human Warrior\n#next 6-10 Sample Hills;6-10 Other\n#author someone\nfree text\nstep\n    .goto Test Vale,1,2\n]])\n";
    let file = rxp::parse(text.as_bytes()).unwrap();
    assert!(file.issues.is_empty());
    let [g] = file.guides.as_slice() else {
        panic!()
    };
    assert_eq!(g.line, 3);
    assert_eq!(g.name(), Some("1-6 Test Vale"));
    let tags: Vec<_> = g.header.iter().map(|h| h.tag.clone()).collect();
    assert!(tags.contains(&HeaderTag::Edition(Edition::Forever)));
    assert!(tags.contains(&HeaderTag::DisplayName("1-6 Vale".into())));
    assert!(tags.contains(&HeaderTag::Version(Some(2))));
    assert!(tags.contains(&HeaderTag::Group("Synthetic".into())));
    assert!(tags.contains(&HeaderTag::Subgroup("Part".into())));
    assert!(tags.contains(&HeaderTag::Next(vec![
        "6-10 Sample Hills".into(),
        "6-10 Other".into()
    ])));
    assert!(
        tags.contains(&HeaderTag::Unsupported),
        "#author is kept as unsupported"
    );
    assert_eq!(g.unsupported_header_lines, [14]);
    let filter = g.filters[0].1.as_ref().unwrap();
    assert_eq!(filter.single_character(), Some(HUMAN_WARRIOR));
    assert_eq!(rxp_verify::default_character(&file), Some(HUMAN_WARRIOR));
}

#[test]
fn parses_every_directive_note_and_step_tag() {
    let text = guide(
        "step << Warrior/Paladin\n    #optional\n    #sticky\n    #label a\n    #completewith next\n    #requires a\n    #level 4\n    #xprate <1.5\n\
         .accept 1 >>Accept\n    .accept -2\n    .turnin 1,2 >>Turn in\n    .turnin -3\n    .complete 1,1 --Label\n    .abandon 1\n\
         .goto Test Vale,48.17,42.94\n    .goto Test Vale,1,2,60,0\n    .waypoint Test Vale,3,4\n    .fly Testford >>Fly\n    .fp Testford\n    .zone Sample Hills\n\
         .train 100\n    .trainer\n    .xp 6\n    .xp 6.5,1\n    .xp <9,1\n    .xp 7+200\n    .xp 7-300\n    .maxlevel 12\n    .home\n    .hs\n    .vendor\n    .mob Practice Dummy\n\
         .use 7\n    .collect 7,4\n    .collect 7,4,1,1,-1\n    .gossipoption 55\n    .deathskip\n    .isOnQuest 1,2\n    .isQuestComplete 1\n    .isQuestTurnedIn 1\n    .isQuestAvailable 1\n\
         .unitscan Someone\n    .accept x\n    .goto Test Vale\n    .xp level\n    .trainer extra\n    >>A note\n    *Window note\n    +Objective note\n    -- comment\n    stray text\n\
         .train 100 >>Warriors only << Warrior\n    .train 100 << !Warrior >>note",
    );
    let file = rxp::parse(text.as_bytes()).unwrap();
    let step = &file.guides[0].steps[0];
    let filter = step.filter.clone().unwrap().unwrap();
    assert_eq!(filter.applies(HUMAN_WARRIOR), Applies::Yes);
    let tags: Vec<_> = step.tags.iter().map(|(_, t)| t.clone()).collect();
    assert_eq!(
        tags,
        [
            StepTag::Optional,
            StepTag::Sticky,
            StepTag::Label("a".into()),
            StepTag::CompleteWith("next".into()),
            StepTag::Requires("a".into()),
            StepTag::Level(Some(4)),
            StepTag::Unsupported,
        ]
    );
    let directives: Vec<_> = step
        .lines
        .iter()
        .filter_map(|l| match &l.item {
            Item::Directive { directive, .. } => Some(directive.clone()),
            _ => None,
        })
        .collect();
    use Directive as D;
    let goto = |x: f64, y: f64| D::Goto {
        zone: "Test Vale".into(),
        x,
        y,
    };
    let expected = vec![
        D::Accept {
            quest: 1,
            conditional: false,
        },
        D::Accept {
            quest: 2,
            conditional: true,
        },
        D::TurnIn {
            quest: 1,
            reward: Some(2),
            conditional: false,
        },
        D::TurnIn {
            quest: 3,
            reward: None,
            conditional: true,
        },
        D::Complete {
            quest: 1,
            objective: 1,
        },
        D::Abandon(1),
        goto(48.17, 42.94),
        goto(1.0, 2.0),
        goto(3.0, 4.0),
        D::Fly("Testford".into()),
        D::FlightPath("Testford".into()),
        D::Zone("Sample Hills".into()),
        D::Train(100),
        D::Trainer,
        D::Xp {
            level: 6,
            kind: XpKind::Reach,
        },
        D::Xp {
            level: 6,
            kind: XpKind::Reach,
        },
        D::Xp {
            level: 9,
            kind: XpKind::Below,
        },
        D::Xp {
            level: 7,
            kind: XpKind::Reach,
        },
        D::Xp {
            level: 7,
            kind: XpKind::Reach,
        },
        D::MaxLevel(12),
        D::Home,
        D::Hearth,
        D::Vendor,
        D::Mob("Practice Dummy".into()),
        D::Use(7),
        D::Collect { item: 7, count: 4 },
        D::Collect { item: 7, count: 4 },
        D::Gossip(55),
        D::DeathSkip,
        D::IsOnQuest(vec![1, 2]),
        D::IsQuestComplete(1),
        D::IsQuestTurnedIn(1),
        D::IsQuestAvailable(1),
        D::Unsupported,
        D::Malformed,
        D::Malformed,
        D::Malformed,
        D::Malformed,
        D::Train(100),
        D::Train(100),
    ];
    assert_eq!(directives, expected);
    let notes: Vec<_> = step
        .lines
        .iter()
        .filter_map(|l| match &l.item {
            Item::Note(kind, text) => Some((*kind, text.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        notes,
        [
            (NoteKind::Default, "A note"),
            (NoteKind::StepWindow, "Window note"),
            (NoteKind::Objective, "Objective note"),
        ]
    );
    assert!(step.lines.iter().any(|l| l.item == Item::Comment));
    assert!(step.lines.iter().any(|l| l.item == Item::UnsupportedText));
    // Line filters: trailing after the note, and before the note.
    let last: Vec<_> = step.lines.iter().rev().take(2).collect();
    let after_note = last[1];
    assert_eq!(
        after_note.filter.as_ref().unwrap().applies(HUMAN_WARRIOR),
        Applies::Yes
    );
    assert!(
        matches!(&after_note.item, Item::Directive { note: Some(n), .. } if n == "Warriors only")
    );
    assert_eq!(
        last[0].filter.as_ref().unwrap().applies(HUMAN_WARRIOR),
        Applies::No
    );
}

#[test]
fn filter_precedence_is_not_then_and_then_or() {
    let parse = |f: &str| {
        let file = rxp::parse(guide(&format!("step << {f}\n    .hs")).as_bytes()).unwrap();
        file.guides[0].steps[0].filter.clone().unwrap().unwrap()
    };
    let mage = Character {
        race: Race::Human,
        class: Class::Mage,
    };
    let dwarf_paladin = Character {
        race: Race::Dwarf,
        class: Class::Paladin,
    };
    // `Human Warrior/Dwarf Paladin` = (Human AND Warrior) OR (Dwarf AND Paladin)
    let f = parse("Human Warrior/Dwarf Paladin");
    assert_eq!(f.applies(HUMAN_WARRIOR), Applies::Yes);
    assert_eq!(f.applies(dwarf_paladin), Applies::Yes);
    assert_eq!(f.applies(mage), Applies::No);
    let f = parse("!Warrior !Mage");
    assert_eq!(f.applies(HUMAN_WARRIOR), Applies::No);
    assert_eq!(f.applies(dwarf_paladin), Applies::Yes);
    assert_eq!(parse("Alliance").applies(mage), Applies::Yes);
    assert_eq!(parse("Horde").applies(mage), Applies::No);
    assert_eq!(parse("Scourge").applies(mage), Applies::No);
    // An unknown token cannot be evaluated: unsupported, never guessed...
    assert_eq!(parse("tbc").applies(mage), Applies::Unsupported);
    // ...unless the conjunction is already false or another branch is true.
    assert_eq!(parse("Warrior tbc").applies(mage), Applies::No);
    assert_eq!(parse("Mage/tbc").applies(mage), Applies::Yes);
}

#[test]
fn reports_structural_issues_with_lines() {
    let text = "local x = 1\n]]\nRXPGuides.RegisterGuide([[\n#name a\nRXPGuides.RegisterGuide([[\n#name b\n]]\nRXPGuides.RegisterGuide([==[\nRXPGuides.RegisterGuide([[\n#name c\n";
    let file = rxp::parse(text.as_bytes()).unwrap();
    let issues: Vec<_> = file.issues.iter().map(|i| (i.line, i.issue)).collect();
    assert_eq!(
        issues,
        [
            (1, StructureIssue::TopLevelCode),
            (2, StructureIssue::StrayCloser),
            (3, StructureIssue::UnclosedBlock),
            (7, StructureIssue::MalformedCloser),
            (8, StructureIssue::TopLevelCode),
            (9, StructureIssue::UnclosedBlock),
        ]
    );
    assert_eq!(file.guides.len(), 1, "only the closed block is a guide");
    let report = rxp_verify::verify(&file, &world(), HUMAN_WARRIOR, None);
    assert_eq!(
        report
            .file_findings
            .iter()
            .filter(|f| f.check == Check::BracketBalance && f.status == Status::Error)
            .count(),
        6
    );
    assert!(!report.passes);
}

#[test]
fn one_line_blocks_and_crlf_are_accepted() {
    let text = "\u{feff}RXPGuides.RegisterGuide([[\r\n#forever\r\n#name a\r\n#version 1\r\n#group g\r\nstep\r\n    .zone Test Vale\r\n]])\r\n";
    let file = rxp::parse(text.as_bytes()).unwrap();
    assert!(file.issues.is_empty());
    clean(&rxp_verify::verify(&file, &world(), HUMAN_WARRIOR, None));
    let file = rxp::parse(b"RXPGuides.RegisterGuide([[#name a]])").unwrap();
    assert_eq!((file.guides.len(), file.issues.len()), (1, 0));
}

#[test]
fn imported_lua_is_data_and_is_never_run() {
    let text = "os.execute('touch /never')\nRXPGuides.RegisterGuide([[\n#forever\n#name a\n#version 1\n#group g\nstep\n    .goto Test Vale,1,1\n    os.exit(1)\n]])\nprint('hi')\n";
    let file = rxp::parse(text.as_bytes()).unwrap();
    assert_eq!(
        file.issues.iter().map(|i| i.issue).collect::<Vec<_>>(),
        [StructureIssue::TopLevelCode, StructureIssue::TopLevelCode]
    );
    assert_eq!(file.guides[0].steps[0].lines[1].item, Item::UnsupportedText);
}

#[test]
fn oversize_and_malformed_input_is_refused_not_truncated() {
    let big = vec![b' '; rxp::MAX_FILE_BYTES + 1];
    assert_eq!(rxp::parse(&big), Err(Rejection::TooLarge));
    let at_limit = vec![b'\n'; rxp::MAX_FILE_BYTES];
    assert!(rxp::parse(&at_limit).is_ok());
    let long = format!("-- {}\n", "x".repeat(rxp::MAX_LINE_BYTES));
    assert_eq!(rxp::parse(long.as_bytes()), Err(Rejection::LineTooLong));
    assert_eq!(rxp::parse(b"\xff\xfe"), Err(Rejection::Encoding));
    assert_eq!(rxp::parse(b"-- a\0b"), Err(Rejection::Encoding));
    let many = "RXPGuides.RegisterGuide([[#name a]])\n".repeat(rxp::MAX_GUIDES + 1);
    assert_eq!(rxp::parse(many.as_bytes()), Err(Rejection::Capacity));
    let steps = guide(&"step\n".repeat(rxp::MAX_STEPS_PER_GUIDE + 1));
    assert_eq!(rxp::parse(steps.as_bytes()), Err(Rejection::Capacity));
    let lines = guide(&format!(
        "step\n{}",
        "    .hs\n".repeat(rxp::MAX_LINES_PER_STEP + 1)
    ));
    assert_eq!(rxp::parse(lines.as_bytes()), Err(Rejection::Capacity));
    let header = format!(
        "RXPGuides.RegisterGuide([[\n{}]])",
        "#group g\n".repeat(rxp::MAX_HEADER_LINES + 1)
    );
    assert_eq!(rxp::parse(header.as_bytes()), Err(Rejection::Capacity));
    // Over-long labels are kept as malformed/unsupported, never cut.
    let label = "Z".repeat(rxp::MAX_LABEL_BYTES + 1);
    let file = rxp::parse(guide(&format!("step\n    .zone {label}")).as_bytes()).unwrap();
    assert!(matches!(
        file.guides[0].steps[0].lines[0].item,
        Item::Directive {
            directive: Directive::Malformed,
            ..
        }
    ));
}

// ---------------------------------------------------------------------------
// Verifier: every check, with an ok and a failing case
// ---------------------------------------------------------------------------

#[test]
fn a_fully_known_route_passes() {
    let report = run(
        &guide(
            "step\n    .goto Test Vale,50,50\n    .accept 1\nstep\n    .goto Test Vale,51,51\n    .turnin 1\n    .accept 2\nstep\n    .fly Testford\n    .turnin 2",
        ),
        &world(),
    );
    clean(&report);
    assert!(report.totals.ok > 0);
    assert_eq!(
        report.totals.errors + report.totals.unverified + report.totals.unsupported,
        0
    );
    assert!(
        !report.facts_are_evidence,
        "synthetic facts are never evidence"
    );
    assert_eq!(report.max_log, 1);
    assert!(report.left_in_log.is_empty());
}

#[test]
fn unknown_facts_are_unverified_never_ok() {
    let report = run(
        &guide(
            "step\n    .goto Nowhere Known,50,50\n    .accept 77\n    .fly Somewhere\n    .train 9\nstep\n    .goto Test Vale,1,1\n    .complete 77,1\n    .turnin 77,1",
        ),
        &Mem::default(),
    );
    assert!(!report.passes);
    assert_eq!(report.totals.errors, 0);
    for check in [
        Check::QuestExists,
        Check::PrerequisiteAny,
        Check::PrerequisiteAll,
        Check::RequiredLevel,
        Check::RaceAllowed,
        Check::ClassAllowed,
        Check::Repeatable,
        Check::Seasonal,
        Check::QuestLogCapacity,
        Check::ObjectiveIndex,
        Check::ObjectivesMarked,
        Check::RewardIndex,
        Check::Zone,
        Check::FlightNode,
        Check::TrainSpell,
    ] {
        assert!(has(&report, check, Status::Unverified), "{check:?}");
    }
    assert_eq!(report.estimated_level, None);
}

#[test]
fn quest_existence() {
    let report = run(
        &guide("step\n    .goto Test Vale,1,1\n    .accept 404\n    .turnin 404"),
        &world(),
    );
    assert!(has(&report, Check::QuestExists, Status::Error));
    assert!(
        !has(&report, Check::TurnInInLog, Status::Error),
        "absent quest stops at existence"
    );
}

#[test]
fn prerequisites_must_be_turned_in_before_accept() {
    let mut facts = world();
    facts.quests.get_mut(&3).unwrap().prerequisites_any = Fact::Known(vec![1, 2]);
    facts.quests.get_mut(&4).unwrap().prerequisites_all = Fact::Known(vec![1, 2]);
    let early = run(
        &guide("step\n    .goto Test Vale,1,1\n    .accept 3\n    .accept 4"),
        &facts,
    );
    assert!(has(&early, Check::PrerequisiteAny, Status::Error));
    assert!(has(&early, Check::PrerequisiteAll, Status::Error));
    let partial = run(
        &guide(
            "step\n    .goto Test Vale,1,1\n    .accept 2\n    .turnin 2\n    .accept 3\n    .accept 4",
        ),
        &facts,
    );
    assert!(
        !has(&partial, Check::PrerequisiteAny, Status::Error),
        "any one suffices"
    );
    assert!(
        has(&partial, Check::PrerequisiteAll, Status::Error),
        "all are needed"
    );
    let late = run(
        &guide(
            "step\n    .goto Test Vale,1,1\n    .accept 1\n    .accept 2\n    .turnin 1\n    .turnin 2\n    .accept 3\n    .accept 4\n    .turnin 3\n    .turnin 4",
        ),
        &facts,
    );
    clean(&late);
}

#[test]
fn required_level_uses_gates_section_names_and_the_xp_estimate() {
    let mut facts = world();
    facts.quests.get_mut(&5).unwrap().required_level = Fact::Known(4);
    let body =
        |gate: &str| format!("step\n    .goto Test Vale,1,1\n{gate}    .accept 5\n    .turnin 5");
    let low = run(&guide(&body("")), &facts);
    assert!(
        has(&low, Check::RequiredLevel, Status::Error),
        "est. level 1 < 4"
    );
    let gated = run(&guide(&body("    .xp 4\n")), &facts);
    clean(&gated);
    let below = run(&guide(&body("    .xp <4,1\n")), &facts);
    assert!(
        has(&below, Check::RequiredLevel, Status::Error),
        "`.xp <N` is no gate"
    );
    // `#name 4-6 ...` declares the section start level.
    let named = guide(&body("")).replace("#name 1-5", "#name 4-6");
    clean(&run(&named, &facts));
    // Reached only through the XP estimate: a warning, not a pass.
    facts.quests.get_mut(&1).unwrap().xp = Fact::Known(2000);
    facts.level_xp = Some(vec![400; 59]);
    let estimated = run(
        &guide("step\n    .goto Test Vale,1,1\n    .accept 1\n    .turnin 1\n    .accept 5"),
        &facts,
    );
    assert!(has(&estimated, Check::RequiredLevel, Status::Warning));
    assert_eq!(estimated.estimated_level, Some(12));
    // Without the level table the estimate is unknown, so the check is unverified.
    facts.level_xp = None;
    let unknown = run(
        &guide("step\n    .goto Test Vale,1,1\n    .accept 1\n    .turnin 1\n    .accept 5"),
        &facts,
    );
    assert!(has(&unknown, Check::RequiredLevel, Status::Unverified));
}

#[test]
fn race_and_class_availability() {
    let mut facts = world();
    facts.quests.get_mut(&1).unwrap().races = Fact::Known(Allowed::Only([Race::Dwarf].into()));
    facts.quests.get_mut(&2).unwrap().classes = Fact::Known(Allowed::Only([Class::Mage].into()));
    facts.quests.get_mut(&3).unwrap().classes =
        Fact::Known(Allowed::Only([Class::Warrior, Class::Paladin].into()));
    let report = run(
        &guide("step\n    .goto Test Vale,1,1\n    .accept 1\n    .accept 2\n    .accept 3"),
        &facts,
    );
    assert!(has(&report, Check::RaceAllowed, Status::Error));
    assert!(has(&report, Check::ClassAllowed, Status::Error));
    let class_errors = report.steps[0]
        .findings
        .iter()
        .filter(|f| f.check == Check::ClassAllowed)
        .count();
    assert_eq!(class_errors, 1, "quest 3 allows Warrior");
}

#[test]
fn repeatable_and_seasonal_quests_are_errors() {
    let mut facts = world();
    facts.quests.get_mut(&1).unwrap().repeatable = Fact::Known(true);
    facts.quests.get_mut(&2).unwrap().seasonal = Fact::Known(true);
    let report = run(
        &guide("step\n    .goto Test Vale,1,1\n    .accept 1\n    .accept 2"),
        &facts,
    );
    assert!(has(&report, Check::Repeatable, Status::Error));
    assert!(has(&report, Check::Seasonal, Status::Error));
}

#[test]
fn quest_log_capacity() {
    let mut facts = world();
    facts.capacity = Some(2);
    let report = run(
        &guide("step\n    .goto Test Vale,1,1\n    .accept 1\n    .accept 2\n    .accept 3"),
        &facts,
    );
    let over: Vec<_> = report.steps[0]
        .findings
        .iter()
        .filter(|f| f.check == Check::QuestLogCapacity)
        .map(|f| f.quest)
        .collect();
    assert_eq!(over, [Some(3)]);
    assert_eq!(report.max_log, 3);
}

#[test]
fn accept_before_turn_in_and_double_turn_ins() {
    let report = run(
        &guide("step\n    .goto Test Vale,1,1\n    .turnin 1"),
        &world(),
    );
    assert!(has(&report, Check::TurnInInLog, Status::Error));
    let double = run(
        &guide("step\n    .goto Test Vale,1,1\n    .accept 1\n    .turnin 1\n    .turnin 1"),
        &world(),
    );
    assert!(has(&double, Check::DoubleTurnIn, Status::Error));
    assert!(!has(&double, Check::TurnInInLog, Status::Error));
    let twice = run(
        &guide("step\n    .goto Test Vale,1,1\n    .accept 1\n    .accept 1"),
        &world(),
    );
    assert!(has(&twice, Check::DoubleAccept, Status::Error));
    let again = run(
        &guide("step\n    .goto Test Vale,1,1\n    .accept 1\n    .turnin 1\n    .accept 1"),
        &world(),
    );
    assert!(has(&again, Check::ReAcceptDone, Status::Error));
}

#[test]
fn objectives_and_rewards() {
    let mut facts = world();
    *facts.quests.get_mut(&1).unwrap() = QuestFacts {
        reward_choices: Fact::Known(2),
        ..quest(2)
    };
    let report = run(
        &guide(
            "step\n    .goto Test Vale,1,1\n    .complete 2,1\n    .accept 1\n    .complete 1,1\n    .complete 1,3\n    .turnin 1,3",
        ),
        &facts,
    );
    assert!(
        has(&report, Check::CompleteInLog, Status::Error),
        "2 not in log"
    );
    assert!(
        has(&report, Check::ObjectiveIndex, Status::Error),
        "objective 3 of 2"
    );
    assert!(
        has(&report, Check::RewardIndex, Status::Error),
        "reward 3 of 2"
    );
    let partial = run(
        &guide("step\n    .goto Test Vale,1,1\n    .accept 1\n    .complete 1,1\n    .turnin 1,2"),
        &facts,
    );
    assert!(
        has(&partial, Check::ObjectivesMarked, Status::Warning),
        "1 of 2 marked"
    );
    assert!(partial.passes, "warnings alone do not fail the replay");
    let full = run(
        &guide(
            "step\n    .goto Test Vale,1,1\n    .accept 1\n    .complete 1,1\n    .complete 1,2\n    .turnin 1,2",
        ),
        &facts,
    );
    clean(&full);
}

#[test]
fn abandon_and_left_in_log() {
    let report = run(
        &guide(
            "step\n    .goto Test Vale,1,1\n    .accept 1\n    .accept 2\n    .abandon 1\n    .abandon 3",
        ),
        &world(),
    );
    assert!(
        has(&report, Check::Abandon, Status::Warning),
        "3 was not in the log"
    );
    assert!(has(&report, Check::LeftInLog, Status::Warning));
    assert_eq!(report.left_in_log, [2]);
}

#[test]
fn zones_coordinates_flight_nodes_and_training() {
    let report = run(
        &guide(
            "step\n    .goto Wrong Vale,1,1\n    .goto Test Vale,101,5\n    .fly Nowhere\n    .fp Nowhere\n    .zone Wrong Vale\n    .train 100\n    .train 101",
        ),
        &world(),
    );
    let errors: Vec<_> = report.steps[0]
        .findings
        .iter()
        .filter(|f| f.status == Status::Error)
        .map(|f| f.check)
        .collect();
    assert_eq!(
        errors,
        [
            Check::Zone,
            Check::Coordinate,
            Check::FlightNode,
            Check::FlightNode,
            Check::Zone
        ]
    );
    assert!(
        has(&report, Check::TrainSpell, Status::Unverified),
        "spell 101 is unknown"
    );
    let mut facts = world();
    facts.spells.insert(101, [Class::Mage].into());
    let report = run(&guide("step\n    .train 101"), &facts);
    assert!(has(&report, Check::TrainSpell, Status::Error));
    // A zone name is accepted as a `.fly` destination, as verify2 does.
    clean(&run(&guide("step\n    .fly Sample Hills"), &world()));
}

#[test]
fn header_hygiene_and_editions() {
    let missing = "RXPGuides.RegisterGuide([[\n#name 1-2 a\nstep\n    .hs\n]])";
    let report = run(missing, &world());
    assert_eq!(
        findings(&report)
            .iter()
            .filter(|f| **f == (Check::HeaderRequired, Status::Error))
            .count(),
        2,
        "group and version"
    );
    assert!(
        has(&report, Check::Edition, Status::Error),
        "no edition tag"
    );
    let doubled = guide("step\n    .hs").replace("#version 1", "#version 1\n#version 2\n#classic");
    let report = run(&doubled, &world());
    assert!(has(&report, Check::HeaderDuplicate, Status::Error));
    assert!(
        has(&report, Check::Edition, Status::Error),
        "two editions in one guide"
    );
    let mixed = format!(
        "{}{}",
        guide("step\n    .hs"),
        guide("step\n    .hs")
            .replace("#forever", "#classic")
            .replace("1-5 Test Vale", "5-6 b")
    );
    let report = run(&mixed, &world());
    assert!(
        report
            .file_findings
            .iter()
            .any(|f| f.check == Check::Edition)
    );
    let same = format!("{}{}", guide("step\n    .hs"), guide("step\n    .hs"));
    assert!(has(&run(&same, &world()), Check::GuideName, Status::Error));
    let next = guide("step\n    .hs").replace("#version 1", "#version 1\n#next 5-6 Elsewhere");
    assert!(has(
        &run(&next, &world()),
        Check::NextGuide,
        Status::Warning
    ));
    let bad_version = guide("step\n    .hs").replace("#version 1", "#version two");
    assert!(has(
        &run(&bad_version, &world()),
        Check::HeaderTag,
        Status::Error
    ));
    let filters = guide("step\n    .hs").replace("<< Human", "<< Human\n<< Warrior");
    assert!(has(
        &run(&filters, &world()),
        Check::GuideFilter,
        Status::Error
    ));
}

#[test]
fn unsupported_items_are_reported_not_dropped() {
    let text = guide("step << tbc\n    #xprate <1.5\n    .unitscan Someone\n    stray words\n    .train 100 >>x << Warrior tbc")
        .replace("#version 1", "#version 1\n#author me\nloose header line");
    let report = run(&text, &world());
    for check in [
        Check::StepFilter,
        Check::StepTag,
        Check::Directive,
        Check::Text,
        Check::HeaderTag,
        Check::HeaderLine,
    ] {
        assert!(has(&report, check, Status::Unsupported), "{check:?}");
    }
    assert!(!report.passes);
    let malformed = run(&guide("step\n    .accept x"), &world());
    assert!(has(&malformed, Check::Directive, Status::Error));
}

#[test]
fn filters_and_conditions_skip_steps() {
    let text = guide(
        "step << Mage\n    .goto Test Vale,1,1\n    .accept 404\nstep\n    .goto Test Vale,1,1\n    .isOnQuest 1\n    .turnin 1\nstep\n    .goto Test Vale,1,1\n    .accept 1\n    .accept 2 << Mage\nstep\n    .goto Test Vale,1,1\n    .isQuestComplete 1\n    .turnin 1\nstep\n    .goto Test Vale,1,1\n    .isQuestTurnedIn 1\n    .isQuestAvailable 3\n    .accept 3\n    .turnin 3",
    );
    let report = run(&text, &world());
    clean(&report);
    use rxp_verify::StepState as S;
    let states: Vec<_> = report.steps.iter().map(|s| s.state).collect();
    assert_eq!(
        states,
        [
            S::Filtered,
            S::Conditional,
            S::Replayed,
            S::Replayed,
            S::Replayed
        ]
    );
    assert_eq!(report.totals.skipped_steps, 2);
    // A guide filtered out for this character replays nothing.
    let mage_guide =
        guide("step\n    .goto Test Vale,1,1\n    .turnin 404").replace("<< Human", "<< Mage");
    let report = run(&mage_guide, &world());
    assert_eq!(report.guides[0].applies, Applies::No);
    clean(&report);
}

#[test]
fn optional_and_conditional_quest_lines() {
    let text = guide(
        "step\n    #optional\n    .goto Test Vale,1,1\n    .accept 1\n    .complete 1,1\n    .turnin 1\nstep\n    .goto Test Vale,1,1\n    .accept -2\n    .turnin -2",
    );
    let report = run(&text, &world());
    clean(&report);
    assert_eq!(
        report.max_log, 0,
        "optional and -id pickups stay out of the default log"
    );
    let optional_absent = guide("step\n    #optional\n    .goto Test Vale,1,1\n    .accept 404");
    assert!(has(
        &run(&optional_absent, &world()),
        Check::QuestExists,
        Status::Error
    ));
}

#[test]
fn placeholder_text_and_step_location() {
    let report = run(
        &guide("step\n    .accept 1 >>Talk to the <TXT>\n    >>Doodad here"),
        &world(),
    );
    let placeholders = findings(&report)
        .iter()
        .filter(|f| **f == (Check::PlaceholderText, Status::Error))
        .count();
    assert_eq!(placeholders, 2);
    assert!(has(&report, Check::StepLocation, Status::Warning));
}

#[test]
fn reports_never_contain_guide_text() {
    let secret = "PRIVATE-NOTE-SENTINEL";
    let text = guide(&format!(
        "step\n    .goto {secret},1,1\n    .accept 1 >>{secret}\n    .{secret}\n    {secret}"
    ))
    .replace("Test Vale", secret);
    let json = serde_json::to_string(&run(&text, &world())).unwrap();
    assert!(!json.contains(secret));
}

// ---------------------------------------------------------------------------
// JSON facts
// ---------------------------------------------------------------------------

fn facts_json(body: &str) -> String {
    format!(
        "{{\"format\":\"navzeroth-guide-facts\",\"version\":1,\"origin\":\"local_observation\"{body}}}"
    )
}

#[test]
fn json_facts_map_known_unknown_and_absent() {
    let text = facts_json(
        ",\"quest_log_capacity\":20,\"level_xp\":[400,900],\
         \"zones\":{\"known\":[\"Test Vale\"],\"complete\":true},\
         \"flight_nodes\":{\"known\":[\"Testford\"],\"complete\":false},\
         \"quests\":{\"complete\":false,\"absent\":[404],\"records\":{\"1\":{\"required_level\":2,\"races\":\"any\",\"classes\":[\"Warrior\"]}}},\
         \"spells\":{\"100\":{\"classes\":[\"Warrior\"]}}",
    );
    let facts = JsonFacts::parse(text.as_bytes()).unwrap();
    assert_eq!(facts.origin, FactOrigin::LocalObservation);
    let QuestRecord::Known(q) = facts.quest(1) else {
        panic!()
    };
    assert_eq!(q.required_level, Fact::Known(2));
    assert_eq!(q.races, Fact::Known(Allowed::Any));
    assert_eq!(
        q.classes,
        Fact::Known(Allowed::Only([Class::Warrior].into()))
    );
    assert_eq!(
        q.objectives,
        Fact::Unknown,
        "absent field is unknown, not zero"
    );
    assert_eq!(facts.quest(404), QuestRecord::Absent);
    assert_eq!(
        facts.quest(2),
        QuestRecord::Unknown,
        "quests are not complete"
    );
    assert_eq!(facts.zone("Test Vale"), Fact::Known(true));
    assert_eq!(
        facts.zone("Other"),
        Fact::Known(false),
        "zones are complete"
    );
    assert_eq!(
        facts.flight_node("Other"),
        Fact::Unknown,
        "nodes are not complete"
    );
    assert_eq!(facts.quest_log_capacity(), Fact::Known(20));
    assert_eq!(facts.level_xp(2), Fact::Known(900));
    assert_eq!(facts.level_xp(3), Fact::Unknown);
    assert_eq!(
        facts.spell_trainable(100, Class::Warrior),
        Fact::Known(true)
    );
    assert_eq!(facts.spell_trainable(100, Class::Mage), Fact::Known(false));
    assert_eq!(facts.spell_trainable(5, Class::Mage), Fact::Unknown);
    let minimal = JsonFacts::parse(facts_json("").as_bytes()).unwrap();
    assert_eq!(minimal.quest(1), QuestRecord::Unknown);
    assert_eq!(minimal.zone("Test Vale"), Fact::Unknown);
    assert_eq!(minimal.quest_log_capacity(), Fact::Unknown);
}

#[test]
fn json_facts_are_strict_and_bounded() {
    let reject = |text: &str| JsonFacts::parse(text.as_bytes()).unwrap_err();
    assert_eq!(
        JsonFacts::parse(&vec![b' '; rxp_verify::MAX_FACTS_BYTES + 1]).unwrap_err(),
        FactsRejection::TooLarge
    );
    assert_eq!(reject("not json"), FactsRejection::Json);
    assert_eq!(
        reject("{\"format\":\"navzeroth-guide-facts\",\"format\":\"x\",\"version\":1}"),
        FactsRejection::Json,
        "duplicate keys"
    );
    assert_eq!(
        reject("{\"format\":\"other\",\"version\":1,\"origin\":\"synthetic\"}"),
        FactsRejection::Format
    );
    assert_eq!(
        reject("{\"format\":\"navzeroth-guide-facts\",\"version\":2,\"origin\":\"synthetic\"}"),
        FactsRejection::Format
    );
    assert_eq!(reject(&facts_json(",\"extra\":1")), FactsRejection::Json);
    assert_eq!(
        reject(&facts_json(
            ",\"quests\":{\"complete\":false,\"records\":{\"1\":{\"guessed\":true}}}"
        )),
        FactsRejection::Json
    );
    assert_eq!(
        reject(&facts_json(
            ",\"quests\":{\"complete\":false,\"records\":{\"1\":{\"races\":[\"Elf\"]}}}"
        )),
        FactsRejection::Json
    );
    assert_eq!(
        reject(&facts_json(
            ",\"quests\":{\"complete\":false,\"records\":{\"01\":{}}}"
        )),
        FactsRejection::Value
    );
    assert_eq!(
        reject(&facts_json(
            ",\"quests\":{\"complete\":false,\"records\":{\"x\":{}}}"
        )),
        FactsRejection::Value
    );
    assert_eq!(
        reject(&facts_json(
            ",\"quests\":{\"complete\":false,\"records\":{\"1\":{\"races\":[]}}}"
        )),
        FactsRejection::Value
    );
    assert_eq!(
        reject(&facts_json(
            ",\"quests\":{\"complete\":false,\"absent\":[1],\"records\":{\"1\":{}}}"
        )),
        FactsRejection::Value
    );
    assert_eq!(
        reject(&facts_json(",\"quest_log_capacity\":0")),
        FactsRejection::Value
    );
    assert_eq!(
        reject(&facts_json(",\"level_xp\":[]")),
        FactsRejection::Value
    );
    assert_eq!(
        reject(&facts_json(
            ",\"zones\":{\"known\":[\"a\",\"a\"],\"complete\":true}"
        )),
        FactsRejection::Value
    );
    assert_eq!(
        reject(&facts_json(
            ",\"zones\":{\"known\":[\" a\"],\"complete\":true}"
        )),
        FactsRejection::Value
    );
    assert_eq!(
        reject(&facts_json(
            ",\"spells\":{\"1\":{\"classes\":[\"Mage\",\"Mage\"]}}"
        )),
        FactsRejection::Value
    );
    let names = (0..=rxp_verify::MAX_FACT_NAMES)
        .map(|i| format!("\"z{i}\""))
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(
        reject(&facts_json(&format!(
            ",\"zones\":{{\"known\":[{names}],\"complete\":true}}"
        ))),
        FactsRejection::Capacity
    );
}
