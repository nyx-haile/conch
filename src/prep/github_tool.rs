pub fn parse_repo_ref(input: &str) -> Option<(String, String)> {
    let trimmed = input.trim();
    let stripped = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("github.com/")
        .trim_end_matches('/')
        .trim_end_matches(".git");
    let parts: Vec<&str> = stripped.split('/').collect();
    if parts.len() < 2 {
        return None;
    }
    let owner = parts[0];
    let repo = parts[1];
    if owner.is_empty()
        || repo.is_empty()
        || owner.contains(' ')
        || repo.contains(' ')
        || !owner.chars().any(|c| c.is_ascii_alphanumeric())
        || !repo.chars().any(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}
