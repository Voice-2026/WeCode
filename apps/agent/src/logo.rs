//! ASCII welcome banner printed when the host starts.

const LOGO: &str = r#"
   ___  ___  ___  _   ___  __
  / __|/ _ \|   \| | | \ \/ /
 | (__| (_) | |) | |_| |>  <
  \___|\___/|___/ \___//_/\_\
"#;

/// Print the welcome banner with the version centered under the wordmark.
pub fn print_banner(version: &str) {
    print!("{LOGO}");
    println!("  headless host · v{version}\n");
}
