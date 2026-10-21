#![cfg(all(debug_assertions, feature = "native"))]

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

#[cfg(feature = "test-support")]
#[gpui::test]
fn missing_assets_error_recommends_the_bundled_public_api(cx: &mut gpui::TestAppContext) {
    let error = cx.update(|cx| gpui_agentation::ely::init(cx).unwrap_err());
    let message = error.to_string();
    assert!(message.contains("gpui_agentation::ely::Assets"));
    assert!(message.contains("gpui_agentation::ely::Assets::before"));
    assert!(!message.contains("ely_gpui_component"));
}
