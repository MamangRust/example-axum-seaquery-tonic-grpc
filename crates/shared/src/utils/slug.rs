pub fn generate_slug(title: &str) -> String {
    let title = title.to_lowercase();

    let mut slug = title.replace(' ', "-");

    slug = slug
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();

    slug = slug.trim_matches('-').to_string();

    slug
}
