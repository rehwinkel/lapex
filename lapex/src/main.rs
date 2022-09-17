fn main() {
    let path = "example/test1.lapex";
    let file_contents = std::fs::read(path).unwrap();
    let (remaining, rules) = lapex_input::parse_lapex(&file_contents).unwrap();
    println!("{}", std::str::from_utf8(remaining).unwrap());
    let (alphabet, dfa) = lapex_lexer::generate_dfa(rules);
}
