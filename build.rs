fn main() {
    println!("cargo:rerun-if-changed=resources");
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "ceedee_ripper.gresource",
    );
}