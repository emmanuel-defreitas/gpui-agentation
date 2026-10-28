#[cfg(feature = "native")]
#[path = "build/ely.rs"]
mod ely;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(agentation_external_inspector)");
    println!("cargo:rerun-if-env-changed=AGENTATION_EXTERNAL_INSPECTOR");
    if std::env::var_os("AGENTATION_EXTERNAL_INSPECTOR").is_some() {
        println!("cargo:rustc-cfg=agentation_external_inspector");
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build/ely.rs");
    #[cfg(feature = "native")]
    ely::generate();
}
