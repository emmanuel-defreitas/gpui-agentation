#[cfg(feature = "native")]
#[path = "build/ely.rs"]
mod ely;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build/ely.rs");
    #[cfg(feature = "native")]
    ely::generate();
}
