use super::*;
use crate::ely::primitives::raise;
use gpui::{Context, Modifiers, Render, TestAppContext, anchored, div, point, prelude::*, px};
use std::{cell::Cell, rc::Rc};

struct TestSurface {
    selecting: bool,
    clicked: Rc<Cell<bool>>,
}

impl Render for TestSurface {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        begin_frame(window, cx);
        let viewport = window.viewport_size();
        let clicked = self.clicked.clone();
        div()
            .relative()
            .size_full()
            .child(target(
                "page",
                "Page",
                div().size(px(300.0)).child(target(
                    "button",
                    "Button",
                    div()
                        .id("real-button")
                        .w(px(100.0))
                        .h(px(40.0))
                        .on_click(move |_, _, _| clicked.set(true)),
                )),
            ))
            .when(self.selecting, |root| {
                root.child(raise(
                    "picker",
                    anchored().position(point(px(0.0), px(0.0))).child(
                        div()
                            .w(viewport.width)
                            .h(viewport.height)
                            .occlude()
                            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                                cx.stop_propagation()
                            }),
                    ),
                ))
            })
    }
}

#[gpui::test]
fn registration_should_preserve_layout_and_pick_the_smallest_target(cx: &mut TestAppContext) {
    let (_, cx) = cx.add_window_view(|_, _| TestSurface {
        selecting: false,
        clicked: Rc::new(Cell::new(false)),
    });
    cx.run_until_parked();
    let target = cx
        .update(|window, cx| pick(window, cx, 20.0, 10.0))
        .unwrap();
    assert_eq!(target.id, "button");
    assert_eq!(
        target.bounds,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 40.0
        }
    );
    assert!(target.source.contains("target.test.rs:"));
    cx.simulate_scale_factor_change(2.0);
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, cx| pick(window, cx, 20.0, 10.0))
            .unwrap()
            .bounds
            .width,
        100.0
    );
}

#[gpui::test]
fn raised_picker_should_block_the_application_click_until_closed(cx: &mut TestAppContext) {
    let clicked = Rc::new(Cell::new(false));
    let (view, cx) = cx.add_window_view(|_, _| TestSurface {
        selecting: true,
        clicked: clicked.clone(),
    });
    cx.run_until_parked();
    cx.simulate_click(point(px(20.0), px(10.0)), Modifiers::none());
    assert!(!clicked.get());
    view.update(cx, |view, cx| {
        view.selecting = false;
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_click(point(px(20.0), px(10.0)), Modifiers::none());
    assert!(clicked.get());
}

struct CachedTarget;
impl Render for CachedTarget {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        target("cached", "Cached target", div().w(px(80.0)).h(px(40.0)))
    }
}

struct CachedSurface {
    child: gpui::Entity<CachedTarget>,
    show: bool,
}
impl Render for CachedSurface {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        begin_frame(window, cx);
        div().size_full().when(self.show, |root| {
            root.child(
                self.child
                    .clone()
                    .cached(gpui::StyleRefinement::default().w(px(100.0)).h(px(100.0))),
            )
        })
    }
}

#[gpui::test]
fn registration_should_survive_cached_replay_and_expire_when_removed(cx: &mut TestAppContext) {
    let (root, cx) = cx.add_window_view(|_, cx| CachedSurface {
        child: cx.new(|_| CachedTarget),
        show: true,
    });
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, cx| pick(window, cx, 10.0, 10.0))
            .unwrap()
            .id,
        "cached"
    );
    root.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, cx| pick(window, cx, 10.0, 10.0))
            .unwrap()
            .id,
        "cached"
    );
    root.update(cx, |root, cx| {
        root.show = false;
        cx.notify();
    });
    cx.run_until_parked();
    assert!(
        cx.update(|window, cx| pick(window, cx, 10.0, 10.0))
            .is_none()
    );
}

struct ShapeSurface {
    app_clicks: Rc<Cell<usize>>,
    note_clicks: Rc<Cell<usize>>,
    toolbar_clicks: Rc<Cell<usize>>,
}

impl Render for ShapeSurface {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let app = self.app_clicks.clone();
        let note = self.note_clicks.clone();
        let toolbar = self.toolbar_clicks.clone();
        let viewport = window.viewport_size();
        let shape = crate::AnnotationShape::Box {
            from: (0.1, 0.1),
            to: (0.8, 0.8),
            color: 0,
        };
        div()
            .size_full()
            .child(
                div()
                    .id("app-control")
                    .size_full()
                    .on_click(move |_, _, _| app.set(app.get() + 1)),
            )
            .child(raise(
                "shape-surface",
                anchored().position(point(px(0.0), px(0.0))).child(
                    div()
                        .id("collapsed-surface")
                        .w(viewport.width)
                        .h(viewport.height)
                        .on_mouse_down(gpui::MouseButton::Left, move |event, window, cx| {
                            let size = window.viewport_size();
                            if shape.hit(
                                (event.position.x.into(), event.position.y.into()),
                                (size.width.into(), size.height.into()),
                                8.0,
                            ) {
                                note.set(note.get() + 1);
                                cx.stop_propagation();
                            }
                        })
                        .child(
                            div()
                                .id("toolbar-control")
                                .absolute()
                                .left(viewport.width * 0.1)
                                .top(viewport.height * 0.5)
                                .size(px(30.0))
                                .occlude()
                                .on_click(move |_, _, _| toolbar.set(toolbar.get() + 1)),
                        ),
                ),
            ))
    }
}

#[gpui::test]
fn collapsed_shape_edges_should_take_clicks_without_blocking_the_app_or_toolbar(
    cx: &mut TestAppContext,
) {
    let app = Rc::new(Cell::new(0));
    let note = Rc::new(Cell::new(0));
    let toolbar = Rc::new(Cell::new(0));
    let (_, cx) = cx.add_window_view(|_, _| ShapeSurface {
        app_clicks: app.clone(),
        note_clicks: note.clone(),
        toolbar_clicks: toolbar.clone(),
    });
    cx.run_until_parked();
    let size = cx.update(|window, _| window.viewport_size());
    cx.simulate_click(
        point(size.width * 0.1, size.height * 0.3),
        Modifiers::none(),
    );
    assert_eq!(note.get(), 1);
    assert_eq!(app.get(), 0);
    cx.simulate_click(
        point(size.width * 0.5, size.height * 0.5),
        Modifiers::none(),
    );
    assert_eq!(note.get(), 1);
    assert_eq!(app.get(), 1);
    cx.simulate_click(
        point(size.width * 0.1 + px(5.0), size.height * 0.5 + px(5.0)),
        Modifiers::none(),
    );
    assert_eq!(toolbar.get(), 1);
    assert_eq!(note.get(), 1);
    assert_eq!(app.get(), 1);
}
