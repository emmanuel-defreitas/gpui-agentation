#[cfg(debug_assertions)]
mod app;

#[cfg(debug_assertions)]
fn main() {
    app::run();
}

#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("The annotation demo requires a debug build: cargo run --example demo");
}
