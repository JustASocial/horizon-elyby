//! Settings > Account's skin panel for an offline name (`horizon_launcher::skins`): the front
//! of the skin and the cape, choosing either from a PNG, and the model.
use super::*;
use gpui::RenderImage;
use horizon_launcher::skins::{self, Model};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

/// A baked preview, or None without one.
type Preview = Option<Arc<RenderImage>>;

#[derive(Default)]
pub(super) struct SkinPanel {
    /// The offline name it is for.
    pub(super) name: String,
    seen: Option<(f32, Preview)>,
    viewer: Option<gpui::Entity<crate::wardrobe_page::viewer::Viewer>>,
    viewer_key: Option<(PathBuf, Option<SystemTime>, Model)>,
    /// What the last choice did, or why not.
    note: Option<Result<String, String>>,
}

/// Points per skin texel in the preview.
const TEXEL: f32 = 6.;

impl Launcher {
    pub(super) fn open_skin(&mut self, name: String) {
        self.skin = match self.skin.take() {
            Some(p) if p.name == name => None,
            _ => Some(SkinPanel {
                name,
                ..SkinPanel::default()
            }),
        };
    }

    fn save_look(&mut self, change: impl FnOnce(&mut skins::Look)) -> Result<(), String> {
        let Some(name) = self.skin.as_ref().map(|p| p.name.clone()) else {
            return Ok(());
        };
        let mut accounts = Accounts::load();
        // The name may have been forgotten while a file dialog was open: keep nothing for it.
        if !accounts
            .offline
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&name))
        {
            return Ok(());
        }
        let mut look = accounts.look(&name);
        change(&mut look);
        accounts.looks.insert(name.to_ascii_lowercase(), look);
        accounts.save()?;
        self.accounts = accounts;
        if let Some(p) = &mut self.skin {
            p.seen = None;
            p.viewer_key = None;
        }
        Ok(())
    }

    fn choose_look_file(&mut self, cape: bool, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(tr("Choose").into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(mut paths))) = paths.await else {
                return;
            };
            let Some(path) = paths.pop() else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                this.take_look_file(path, cape);
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn take_look_file(&mut self, path: PathBuf, cape: bool) {
        let Some(name) = self.skin.as_ref().map(|p| p.name.clone()) else {
            return;
        };
        // The name may have been forgotten while the file dialog was open.
        if !Accounts::load()
            .offline
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&name))
        {
            return;
        }
        let done = skins::keep(&skins::dir(), &name, &path, cape).and_then(|()| {
            self.save_look(|l| match cape {
                false => l.skin = true,
                true => l.cape = true,
            })
        });
        if let Some(p) = &mut self.skin {
            let said = match cape {
                false => trf("{name}'s skin is kept.", &[("name", &name)]),
                true => trf("{name}'s cape is kept.", &[("name", &name)]),
            };
            p.note = Some(done.map(|()| said));
        }
    }

    fn previews(&mut self) -> Preview {
        let scale = self.scale;
        let Some(panel) = &mut self.skin else {
            return None;
        };
        if let Some((s, cape)) = &panel.seen {
            if *s == scale {
                return cape.clone();
            }
        }
        let look = self.accounts.look(&panel.name);
        let (_, cape_file) = skins::files(&skins::dir(), &panel.name);
        let read = |f: &PathBuf| std::fs::read(f).ok();
        let cape = read(&cape_file)
            .filter(|_| look.cape)
            .and_then(|b| skins::read_cape(&b).ok())
            .map(|png| art::bake(&skins::cape_front(&png), 10, 16, TEXEL, scale));
        panel.seen = Some((scale, cape.clone()));
        cape
    }

    fn skin_viewer(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Entity<crate::wardrobe_page::viewer::Viewer>> {
        let panel = self.skin.as_ref()?;
        let look = self.accounts.look(&panel.name);
        if !look.skin {
            return None;
        }
        let (path, _) = skins::files(&skins::dir(), &panel.name);
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        let key = (path.clone(), modified, look.model);
        let bytes = std::fs::read(&path).ok()?;
        let image = skins::read_skin(&bytes).ok()?;
        let image = modern_skin(image);
        let texture = horizon_cosmetics::Texture {
            width: 64,
            height: 64,
            rgba: image.into_raw(),
        };
        let viewer = if let Some(viewer) = self.skin.as_ref()?.viewer.clone() {
            viewer
        } else {
            let viewer = cx.new(|_| crate::wardrobe_page::viewer::Viewer::new());
            self.skin.as_mut()?.viewer = Some(viewer.clone());
            viewer
        };
        if self.skin.as_ref()?.viewer_key.as_ref() != Some(&key) {
            viewer.update(cx, |view, cx| {
                view.dress(
                    crate::wardrobe_page::viewer::Outfit {
                        skin: Arc::new(texture),
                        slim: look.model == Model::Slim,
                        items: Vec::new(),
                    },
                    false,
                    cx,
                );
            });
            if let Some(panel) = &mut self.skin {
                panel.viewer_key = Some(key);
            }
        }
        Some(viewer)
    }

    /// The panel, under the offline names while open.
    pub(super) fn skin_panel(&mut self, cx: &mut Context<Self>) -> Option<Div> {
        let t = self.theme;
        let name = self.skin.as_ref()?.name.clone();
        if !self.accounts.offline.iter().any(|n| n == &name) {
            self.skin = None;
            return None;
        }
        let look = self.accounts.look(&name);
        let cape = self.previews();
        let skin_viewer = self
            .skin_viewer(cx)
            .map(|viewer| crate::wardrobe_page::viewer::frame(viewer, false, |_, _| {}).into_any_element());
        let frame = |w: f32, h: f32, image: Preview, empty: &'static str| {
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(w + 24.))
                .h(px(h + 24.))
                .bg(t.well)
                .border_2()
                .border_color(t.outline)
                .child(match image {
                    Some(i) => gpui::img(i).w(px(w)).h(px(h)).into_any_element(),
                    None => text(theme::CAPTION)
                        .text_color(t.muted)
                        .child(tr(empty))
                        .into_any_element(),
                })
        };
        let weak = cx.entity().downgrade();
        let model = segments(
            "skin-model",
            &[(Model::Classic, "Classic arms"), (Model::Slim, "Slim arms")],
            look.model,
            &t,
            move |m, _, cx| {
                let _ = weak.update(cx, |this, cx| {
                    let _ = this.save_look(|l| l.model = m);
                    cx.notify();
                });
            },
        );
        let pick = |id: &'static str, label: &'static str, cape: bool, cx: &mut Context<Self>| {
            button(id, label, Kind::Secondary, &t).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.choose_look_file(cape, cx);
                    cx.notify();
                },
            ))
        };
        let drop = |id: &'static str, label: &'static str, cape: bool, cx: &mut Context<Self>| {
            button(id, label, Kind::Ghost, &t).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    let _ = this.save_look(|l| match cape {
                        false => l.skin = false,
                        true => l.cape = false,
                    });
                    cx.notify();
                },
            ))
        };
        let mut skin_buttons =
            div()
                .flex()
                .gap(px(6.))
                .child(pick("skin-choose", "Choose a skin…", false, cx));
        if look.skin {
            skin_buttons = skin_buttons.child(drop("skin-remove", "Default skin", false, cx));
        }
        let mut cape_buttons =
            div()
                .flex()
                .gap(px(6.))
                .child(pick("cape-choose", "Choose a cape…", true, cx));
        if look.cape {
            cape_buttons = cape_buttons.child(drop("cape-remove", "No cape", true, cx));
        }
        let note = self.skin.as_ref().and_then(|p| p.note.clone());
        Some(
            plate(&t)
                .flex()
                .gap(px(20.))
                .px(px(16.))
                .py(px(14.))
                .ml(px(40.))
                .child(skin_viewer.unwrap_or_else(|| {
                    frame(16. * TEXEL, 32. * TEXEL, None, "Default").into_any_element()
                }))
                .child(frame(10. * TEXEL, 16. * TEXEL, cape, "No cape"))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap(px(12.))
                        .child(caps(trf("{name}'s skin", &[("name", &name)])).text_color(t.muted))
                        .child(skin_buttons)
                        .child(model)
                        .child(cape_buttons)
                        .children(note.map(|n| match n {
                            Ok(said) => text(theme::CAPTION).text_color(t.muted).child(said),
                            Err(e) => text(theme::CAPTION).text_color(t.danger).child(e),
                        }))
                        .child(text(theme::CAPTION).text_color(t.muted).child(tr(
                            "A 64×64 (or 64×32) skin PNG is saved for this offline name and used in Horizon when it plays. The 3D preview uses the selected arm model. Capes are saved here for future client support.",
                        ))),
                ),
        )
    }
}

fn modern_skin(skin: image::RgbaImage) -> image::RgbaImage {
    if skin.height() == 64 {
        return skin;
    }
    let mut modern = image::RgbaImage::new(64, 64);
    for y in 0..32 {
        for x in 0..64 {
            modern.put_pixel(x, y, *skin.get_pixel(x, y));
        }
    }
    for y in 0..12 {
        for x in 0..4 {
            modern.put_pixel(36 + x, 52 + y, *skin.get_pixel(44 + (3 - x), 20 + y));
            modern.put_pixel(20 + x, 52 + y, *skin.get_pixel(4 + (3 - x), 20 + y));
        }
    }
    modern
}

#[cfg(test)]
mod tests {
    use super::modern_skin;

    #[test]
    fn legacy_skin_becomes_a_classic_64_by_64_texture() {
        let old = image::RgbaImage::from_fn(64, 32, |x, y| {
            image::Rgba([x as u8, y as u8, 0, 255])
        });
        let current = modern_skin(old);
        assert_eq!(current.dimensions(), (64, 64));
        assert_eq!(current.get_pixel(36, 52).0, [47, 20, 0, 255]);
        assert_eq!(current.get_pixel(20, 52).0, [7, 20, 0, 255]);
    }
}
