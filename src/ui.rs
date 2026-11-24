//! Agentation presentation helpers composed with GPUI Kit.
#![allow(unused_imports)]
pub(crate) use gpui::assets::IconName;
pub(crate) use gpui::base::FocusTrapElement;
pub(crate) use gpui::component::button::{Button, ButtonCustomVariant, ButtonVariants};
pub(crate) use gpui::component::collapsible::Collapsible;
pub(crate) use gpui::component::input::{Editor, EditorState, Textarea, TextareaState};
pub(crate) use gpui::component::separator::Separator as Divider;
pub(crate) use gpui::component::switch::Switch;
pub(crate) use gpui::component::{
    ActiveTheme, Colorize, Disableable, Icon, Selectable, Sizable, Theme, ThemeMode,
};
use gpui::{prelude::*, *};

#[derive(Clone, Copy)]
pub(crate) enum ControlSize {
    Sm,
    Lg,
}
pub(crate) trait AnnotationMetrics {
    fn control_height(&self, size: ControlSize) -> Rems;
}
impl AnnotationMetrics for Theme {
    fn control_height(&self, size: ControlSize) -> Rems {
        rems(match size {
            ControlSize::Sm => 1.5,
            ControlSize::Lg => 2.0,
        })
    }
}

pub(crate) fn raise(_id: impl Into<ElementId>, child: impl IntoElement) -> gpui::Deferred {
    gpui::deferred(child)
}

/// Observe layout changes without making the measuring canvas a pick target.
#[derive(IntoElement)]
pub(crate) struct BoundsObserver {
    id: ElementId,
    body: Div,
    changed: Box<dyn Fn(Bounds<Pixels>, &mut Window, &mut App)>,
}
impl BoundsObserver {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        changed: impl Fn(Bounds<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            body: div(),
            changed: Box::new(changed),
        }
    }
}
impl ParentElement for BoundsObserver {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(children);
    }
}
impl Styled for BoundsObserver {
    fn style(&mut self) -> &mut StyleRefinement {
        self.body.style()
    }
}
impl RenderOnce for BoundsObserver {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let previous = window.use_keyed_state(self.id, cx, |_, _| None::<Bounds<Pixels>>);
        self.body.relative().child(
            canvas(
                move |bounds, window, cx| {
                    if *previous.read(cx) != Some(bounds) {
                        previous.update(cx, |last, _| *last = Some(bounds));
                        (self.changed)(bounds, window, cx);
                        window.request_animation_frame();
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}

pub(crate) mod motion {
    use gpui::App;
    use gpui::component::ActiveTheme;
    use std::time::Duration;
    pub const FAST: Duration = Duration::from_millis(120);
    pub const BASE: Duration = Duration::from_millis(180);
    pub fn duration(requested: Duration, cx: &App) -> Duration {
        let value = if cx.reduce_motion() {
            Duration::ZERO
        } else if requested == FAST {
            cx.theme().motion.duration_fast
        } else {
            cx.theme().motion.duration_normal
        };
        value.max(Duration::from_millis(1))
    }
    pub fn ease_out_cubic(t: f32) -> f32 {
        1. - (1. - t).powi(3)
    }
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }
}

/// A private window with a real Kit Root and a typed Agentation content view.
pub(crate) struct ToolWindow<V: 'static> {
    root: WindowHandle<gpui::base::Root>,
    marker: std::marker::PhantomData<fn() -> V>,
}
impl<V: 'static> Copy for ToolWindow<V> {}
impl<V: 'static> Clone for ToolWindow<V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<V: 'static> From<ToolWindow<V>> for AnyWindowHandle {
    fn from(window: ToolWindow<V>) -> Self {
        window.root.into()
    }
}
impl<V: 'static> ToolWindow<V> {
    pub fn from_window(window: &Window) -> Self {
        Self {
            root: window
                .window_handle()
                .downcast::<gpui::base::Root>()
                .expect("Agentation window has Kit Root"),
            marker: std::marker::PhantomData,
        }
    }
    pub fn window_id(&self) -> WindowId {
        self.root.window_id()
    }
    pub fn update<C: AppContext, R>(
        &self,
        cx: &mut C,
        update: impl FnOnce(&mut V, &mut Window, &mut Context<V>) -> R,
    ) -> gpui::Result<R> {
        self.root.update(cx, |root, window, cx| {
            let view = root
                .view()
                .clone()
                .downcast::<V>()
                .expect("Agentation content type");
            view.update(cx, |view, cx| update(view, window, cx))
        })
    }
}
pub(crate) fn open_tool_window<V: Render>(
    cx: &mut App,
    options: WindowOptions,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> gpui::Result<ToolWindow<V>> {
    cx.open_window(options, |window, cx| {
        let content = build(window, cx);
        cx.new(|cx| gpui::base::Root::new(content, window, cx))
    })
    .map(|root| ToolWindow {
        root,
        marker: std::marker::PhantomData,
    })
}

/// Load an annotation draft with the caret at its end, without an undo entry.
pub(crate) trait CommentText {
    fn set_comment(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        Self: Sized;
}
impl CommentText for TextareaState {
    fn set_comment(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = text.into();
        let text: String = text.chars().take(8000).collect();
        let len = text.len();
        self.set_value(text, window, cx);
        self.set_selected_range(len..len, cx);
    }
}
