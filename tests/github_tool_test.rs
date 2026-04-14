use conch::prep::github_tool::parse_repo_ref;

#[test]
fn parses_https_url() {
    let r = parse_repo_ref("https://github.com/user/proj").unwrap();
    assert_eq!(r, ("user".to_string(), "proj".to_string()));
}

#[test]
fn parses_url_with_trailing_slash_and_git_suffix() {
    let r = parse_repo_ref("https://github.com/user/proj.git/").unwrap();
    assert_eq!(r, ("user".to_string(), "proj".to_string()));
}

#[test]
fn parses_short_form() {
    let r = parse_repo_ref("user/proj").unwrap();
    assert_eq!(r, ("user".to_string(), "proj".to_string()));
}

#[test]
fn rejects_non_github_input() {
    assert!(parse_repo_ref("howtowin.lol").is_none());
    assert!(parse_repo_ref("just some freeform text").is_none());
}
