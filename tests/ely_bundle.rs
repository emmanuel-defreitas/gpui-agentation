#![cfg(feature = "native")]

#[path = "../build/ely.rs"]
#[allow(dead_code)]
mod bundle;

#[test]
fn bundled_assets_load_with_the_preserved_upstream_keys() {
    use gpui::AssetSource;

    let assets = gpui_agentation::ely::Assets;
    assert!(
        assets
            .load("fonts/Inter-Regular.ttf")
            .unwrap()
            .unwrap()
            .len()
            > 1000
    );
    let icon = assets.load("icons/plus.svg").unwrap().unwrap();
    assert!(std::str::from_utf8(&icon).unwrap().contains("<svg"));
}
