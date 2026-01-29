fn main() {
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/ceedee_ripper.gresource.xml",
        "ceedee_ripper.gresource",
    );
}