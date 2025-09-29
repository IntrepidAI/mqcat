pub fn format_table(info: &[(&str, String)]) -> String {
    let max_key_len = info.iter().map(|(key, _)| key.len()).max().unwrap_or(0) + 2;
    let mut table = String::new();
    for (key, value) in info {
        if key.is_empty() && value.is_empty() {
            table.push('\n');
            continue;
        }
        table.push_str(&format!("{:>max_key_len$}: {}\n", key, value));
    }
    table
}
