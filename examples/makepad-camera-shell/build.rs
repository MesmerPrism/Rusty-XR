fn main() {
    const KEY: &str = "RUSTY_QUEST_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING";
    const DEFAULT: &str = "display-left-from-left-source";

    println!("cargo:rerun-if-env-changed={KEY}");

    let requested = std::env::var(KEY).unwrap_or_else(|_| DEFAULT.to_string());
    let mapping = match requested.as_str() {
        "display-left-from-left-source" => "display-left-from-left-source",
        "display-left-from-right-source" => "display-left-from-right-source",
        _ => DEFAULT,
    };
    println!("cargo:rustc-env={KEY}={mapping}");
}
