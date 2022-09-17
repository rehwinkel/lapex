fn main() {
    let path = "example/test1.lapex";
    let file_contents = std::fs::read(path).unwrap();
    let (remaining, rules) = lapex_input::parse_lapex(&file_contents).unwrap();
    assert!(remaining.is_empty(), "Didn't finish parsing file: \n {}", std::str::from_utf8(remaining).unwrap());
    let mut token_rules = Vec::new();
    let mut prod_rules = Vec::new();
    let mut entry_rules = Vec::new();
    for rule in rules {
        match rule {
            lapex_input::Rule::TokenRule(tr) => token_rules.push(tr),
            lapex_input::Rule::ProductionRule(pr) => prod_rules.push(pr),
            lapex_input::Rule::EntryRule(er) => entry_rules.push(er),
        }
    }
    assert_eq!(entry_rules.len(), 1);
    let _entry_rule = entry_rules.remove(0); //TODO use
    let (alphabet, _dfa) = lapex_lexer::generate_dfa(token_rules);
    for (i, range) in alphabet.iter().enumerate() {
        eprintln!("{} {:?}", i, range);
    }
}
