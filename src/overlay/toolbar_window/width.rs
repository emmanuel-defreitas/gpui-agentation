//! Interruptible native-window width transitions for the external inspector.
use super::*;
use crate::ely::motion;

pub(super) struct WidthAnimation {
    pub current: Pixels,
    target: Pixels,
    visible: bool,
    pub inspector: Option<Entity<gpui::Inspector>>,
    pub opacity: f32,
    generation: u64,
    reveal_pending: bool,
    reveal_task: Option<Task<()>>,
    task: Option<Task<()>>,
}

impl WidthAnimation {
    pub fn new(width: Pixels, visible: bool, inspector: Option<Entity<gpui::Inspector>>) -> Self {
        Self {
            current: width,
            target: width,
            visible,
            inspector,
            opacity: 1.,
            generation: 0,
            reveal_pending: false,
            reveal_task: None,
            task: None,
        }
    }
}

fn sample(from: Pixels, to: Pixels, elapsed: Duration) -> (Pixels, bool) {
    if elapsed >= motion::BASE {
        return (to, false);
    }
    let t = (elapsed.as_secs_f32() / motion::BASE.as_secs_f32()).min(1.);
    (from + (to - from) * motion::ease_out_cubic(t), t < 1.)
}

impl ToolbarWindow {
    pub(super) fn retarget_width(
        &mut self,
        target: Pixels,
        visible: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(inspector) = &self.state.inspector {
            self.width.inspector = Some(inspector.clone());
        }
        let animate =
            visible && self.width.visible && !cx.theme().reduced_motion && !cx.reduce_motion();
        self.width.visible = visible;
        if !animate {
            self.width.task = None;
            self.width.reveal_task = None;
            self.width.generation += 1;
            self.width.reveal_pending = false;
            self.width.opacity = 1.;
            self.width.current = target;
            self.width.target = target;
            self.width.inspector = self.state.inspector.clone();
            return;
        }
        if self.width.target == target {
            return;
        }
        // Dropping the old task cancels it. Start from the last presented width,
        // including when the user reverses direction before reaching an endpoint.
        self.width.task = None;
        self.width.reveal_task = None;
        self.width.generation += 1;
        self.width.reveal_pending = false;
        self.width.opacity = 0.;
        self.width.target = target;
        if self.width.current == target {
            self.width.inspector = self.state.inspector.clone();
            self.confirm_content_reveal(window, cx);
            return;
        }
        let from = self.width.current;
        // Present the transparent interior before changing the native frame.
        let (hidden_frame, hidden_ready) = futures::channel::oneshot::channel();
        window.on_next_frame(move |_, _| {
            let _ = hidden_frame.send(());
        });
        let host = self.host;
        let child = window.window_handle().downcast::<ToolbarWindow>().unwrap();
        self.width.task = Some(cx.spawn(async move |_, cx| {
            if hidden_ready.await.is_err() {
                return;
            }
            let start = cx.background_executor().now();
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let elapsed = cx.background_executor().now() - start;
                let running = host
                    .update(cx, |_, host, cx| {
                        child
                            .update(cx, |this, window, cx| {
                                let (width, running) =
                                    if cx.theme().reduced_motion || cx.reduce_motion() {
                                        (target, false)
                                    } else {
                                        sample(from, target, elapsed)
                                    };
                                this.width.current = width;
                                if !running {
                                    this.width.inspector = this.state.inspector.clone();
                                }
                                #[cfg(target_os = "macos")]
                                let placed =
                                    native::place(host, window, width, this.width.visible, cx);
                                #[cfg(not(target_os = "macos"))]
                                let placed = false;
                                if !placed {
                                    window.resize(size(width, host.bounds().size.height));
                                }
                                if !running {
                                    this.confirm_content_reveal(window, cx);
                                }
                                cx.notify();
                                running
                            })
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                if !running {
                    break;
                }
            }
        }));
    }

    /// Native placement is queued outside App's borrow. Do not reveal merely
    /// because the timer ended: wait for the actual viewport and another frame.
    pub(super) fn confirm_content_reveal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.width.opacity >= 1.
            || self.width.reveal_pending
            || self.width.current != self.width.target
        {
            return;
        }
        if (window.viewport_size().width - self.width.target).abs() > px(1. / window.scale_factor())
        {
            return;
        }
        self.width.reveal_pending = true;
        let generation = self.width.generation;
        let owner = cx.entity().downgrade();
        window.on_next_frame(move |window, cx| {
            let _ = owner.update(cx, |this, cx| {
                if this.width.generation != generation {
                    return;
                }
                this.width.reveal_pending = false;
                if this.width.current != this.width.target
                    || (window.viewport_size().width - this.width.target).abs()
                        > px(1. / window.scale_factor())
                {
                    return;
                }
                this.width.reveal_pending = true;
                let start = cx.background_executor().now();
                let from = this.width.opacity;
                this.width.reveal_task = Some(cx.spawn(async move |owner, cx| {
                    loop {
                        cx.background_executor()
                            .timer(Duration::from_millis(16))
                            .await;
                        let elapsed = cx.background_executor().now() - start;
                        let running = owner
                            .update(cx, |this, cx| {
                                if this.width.generation != generation {
                                    return false;
                                }
                                let t = if cx.theme().reduced_motion || cx.reduce_motion() {
                                    1.
                                } else {
                                    (elapsed.as_secs_f32() / motion::FAST.as_secs_f32()).min(1.)
                                };
                                this.width.opacity = from + (1. - from) * motion::ease_out_cubic(t);
                                cx.notify();
                                t < 1.
                            })
                            .unwrap_or(false);
                        if !running {
                            break;
                        }
                    }
                }));
            });
        });
    }
}
