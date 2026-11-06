//! The owner gate in the window (`horizon_launcher::account::launch`): the screen that asks for a
//! Ely.by sign-in for players who choose an account for online servers.
use super::*;

impl Launcher {
    /// Whether the screen covers the window: no owner, and either Play was refused or the player
    /// hasn't read it yet. The data question comes first.
    pub(crate) fn asking_owner(&self) -> bool {
        self.owner_shown() == Some(false)
            && (self.owner_screen_open || !self.settings.owner_notice_seen)
            && !self.asking_consent()
            && self.opening.is_none()
    }

    /// `owner` as the window shows it: the developer option that lets offline names play without
    /// an owner (`dev_options`) hides the gate, as if the keychain hadn't answered yet.
    pub(crate) fn owner_shown(&self) -> Option<bool> {
        match self.owner {
            Some(false) => None,
            owner => owner,
        }
    }

    /// Opens the screen and starts the sign-in on it.
    pub(crate) fn sign_in_to_play(&mut self) {
        self.owner_screen_open = true;
        self.start_sign_in();
    }

    /// Closes it; it comes back with the next refused Play.
    fn close_owner_screen(&mut self) {
        self.cancel_sign_in();
        self.owner_screen_open = false;
        if !self.settings.owner_notice_seen {
            self.settings.owner_notice_seen = true;
            self.save_settings();
        }
    }

    pub(crate) fn owner_screen(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = self.theme;
        let n = t.night;
        let configured = account::ely::configured();
        let state = match &self.sign_in_state {
            _ if !configured => Shown::Error(tr("Ely.by sign-in is not available in this build.")),
            Some(SignInState::Starting) => Shown::Starting("Preparing Ely.by sign-in…"),
            Some(SignInState::ElyAuthorize(_)) => {
                Shown::Starting("Waiting for Ely.by authorization…")
            }
            Some(SignInState::Working(phase)) => Shown::Starting(phase.label()),
            Some(SignInState::Done(Err(e))) => Shown::Error(e),
            Some(SignInState::Done(Ok(_))) | None => Shown::Idle,
        };
        if crate::glass_ui::on() {
            return self.glass_owner_screen(state, configured, cx);
        }
        let line = |s: &str, color: u32| {
            shell::text(theme::night::UI, shell::c(color))
                .relative()
                .child(tr(s).to_string())
        };
        let mut body = div()
            .relative()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                shell::text(theme::night::CAPTION, shell::c(n.muted))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(tr("Ely.by account").to_string()),
            )
            .child(
                shell::text(theme::night::TITLE, shell::c(n.ink))
                    .mb(px(4.))
                    .child(tr("Sign in to play").to_string()),
            )
            .child(line(
                "Sign in with Ely.by to play online with your account. Guest play works offline without an account.",
                n.ink_soft,
            ))
            .child(line(
                "Ely.by sign-in uses your registered OAuth application.",
                n.ink_soft,
            ))
            .child(line(
                "Your worlds, offline names and settings stay where they are.",
                n.muted,
            ));
        let raised = || {
            div()
                .relative()
                .mt(px(10.))
                .p(px(18.))
                .rounded(px(theme::night::RADIUS_CONTROL))
                .bg(plate::alpha(n.card_hi, n.card_hi_alpha))
                .border_1()
                .border_color(plate::alpha(n.stroke, n.stroke_alpha))
        };
        body = match state {
            Shown::Starting(what) => body.child(raised().child(
                shell::text(theme::night::UI, shell::c(n.ink_soft)).child(tr(what).to_string()),
            )),
            Shown::Error(e) => body.child(
                raised()
                    .child(shell::text(theme::night::UI, shell::c(n.danger)).child(e.to_string())),
            ),
            Shown::Idle => body,
        };
        let main = match state {
            Shown::Starting(_) => None,
            Shown::Error(_) if !configured => None,
            _ => Some(
                shell::action("owner-sign-in", shell::Action::Main, &t)
                    .child(shell::label(tr("Sign in with Ely.by").to_string()))
                    .child(ely_logo::logo(18.))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.sign_in_to_play();
                        cx.notify();
                    })),
            ),
        };
        let answers = div()
            .relative()
            .flex()
            .items_center()
            .gap(px(10.))
            .mt(px(14.))
            .children(main)
            .child(
                shell::action("owner-later", shell::Action::Quiet, &t)
                    .child(shell::label(tr("Not now").to_string()))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.close_owner_screen();
                        cx.notify();
                    })),
            );
        // Opaque: it sits over the page's content, not over the bare sky.
        let card = plate::cut(
            theme::night::PIXEL_CORNER,
            2,
            Some(plate::alpha(n.card, 0.98)),
            Some((plate::alpha(n.stroke, 2. * n.stroke_alpha), 1.)),
        )
        .flex()
        .flex_col()
        .gap(px(8.))
        // A set width: text measured at it wraps where it is drawn.
        .w(px(520.))
        .max_w_full()
        .p(px(32.))
        .child(body)
        .child(answers);
        // The sky stays behind, dimmed: the sheet is over the launcher, not instead of it.
        div()
            .id("owner-gate")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .px(px(32.))
            .py(px(24.))
            .bg(plate::alpha(n.page, 0.8))
            .on_scroll(|_, _, cx| cx.stop_propagation())
            .occlude()
            .child(card)
    }

    /// The account screen in the glass look.
    fn glass_owner_screen(
        &self,
        state: Shown,
        configured: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        use crate::glass_ui::{self as ui, white};
        use horizon_theme::glass::{ink, radius};
        let t = self.theme;
        let panel = || {
            div()
                .mt(px(20.))
                .p(px(20.))
                .rounded(px(radius::POPOVER))
                .bg(white(0.045))
        };
        let mut card = ui::dialog_plate(radius::MODAL)
            .id("owner-card")
            .occlude()
            .flex()
            .flex_col()
            // A set width: text measured at it wraps where it is drawn.
            .w(px(520.))
            .max_w_full()
            .p(px(32.))
            .child(ui::text(13., FontWeight::MEDIUM, ink::QUIET).child(tr("Ely.by account")))
            .child(
                div()
                    .mt(px(8.))
                    .font_family(horizon_theme::glass::DISPLAY_FONT)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_size(px(28.))
                    .line_height(px(35.))
                    .text_color(white(ink::FULL))
                    .child(tr("Sign in to play")),
            )
            .child(
                div()
                    .mt(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(ui::text(14., FontWeight::NORMAL, ink::BODY).child(tr(
                        "Sign in with Ely.by to play online with your account. Guest play works offline without an account.",
                    )))
                    .child(ui::text(14., FontWeight::NORMAL, ink::QUIET).child(tr(
                        "Your worlds, offline names and settings stay where they are.",
                    ))),
            );
        card = match state {
            Shown::Starting(what) => card
                .child(panel().child(ui::text(14., FontWeight::NORMAL, ink::BODY).child(tr(what)))),
            Shown::Error(e) => card.child(
                panel().child(
                    ui::text(14., FontWeight::NORMAL, ink::FULL)
                        .text_color(ui::color(horizon_theme::glass::DANGER))
                        .child(e.to_string()),
                ),
            ),
            Shown::Idle => card,
        };
        let main: Option<AnyElement> = match state {
            Shown::Starting(_) => None,
            Shown::Error(_) if !configured => None,
            _ => Some(
                keyboard(
                    div()
                        .id("owner-sign-in")
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(10.))
                        .h(px(44.))
                        .px(px(20.))
                        .rounded(px(radius::ROW))
                        .bg(gpui::white())
                        .cursor_pointer()
                        .hover(|s| s.opacity(0.9))
                        .font_family(horizon_theme::glass::FONT)
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(gpui::rgb(horizon_theme::glass::ON_LIGHT))
                        .child(ely_logo::logo(18.))
                        .child(tr("Sign in with Ely.by")),
                    &t,
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.sign_in_to_play();
                    cx.notify();
                }))
                .into_any_element(),
            ),
        };
        let later = ui::button("owner-later".into(), tr("Not now").into(), Kind::Ghost, &t)
            .h(px(44.))
            .px(px(20.))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.close_owner_screen();
                cx.notify();
            }));
        card = card.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .mt(px(24.))
                .children(main)
                .child(later),
        );
        div()
            .id("owner-gate")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p(px(24.))
            .on_scroll(|_, _, cx| cx.stop_propagation())
            .occlude()
            .child(ui::scrim_backdrop())
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .bg(ui::scrim()),
            )
            .child(card)
    }
}

/// What the sheet shows of the sign-in.
#[derive(Clone, Copy)]
enum Shown<'a> {
    Idle,
    Starting(&'static str),
    Error(&'a str),
}
