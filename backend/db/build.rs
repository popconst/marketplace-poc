// `sqlx::migrate!` only tracks existing migration files; this rebuilds when one is added.
fn main() {
    println!("cargo::rerun-if-changed=../migrations");
}
