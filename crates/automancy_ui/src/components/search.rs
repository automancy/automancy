use crate::*;

/// Draws a search bar.
#[track_caller]
pub fn searchable_ids<'a, 'b, Id: Copy + Eq + 'static>(
    current_id: &mut Option<Id>,
    ids: (impl Iterator<Item = &'b Id> + Clone),
    text_buffer: &mut String,
    hint_text: impl Into<Option<Cow<'static, str>>>,
    mut draw: impl FnMut(Id),
    mut get_name: impl FnMut(Id) -> &'a str,
) {
    let matcher = use_state(|| fuzzy_matcher::skim::SkimMatcherV2::default().use_cache(true).ignore_case());

    textbox(text_buffer, TextStyle::normal(), hint_text, None);

    Pad::vertical(sizing::PADDING_LARGE).show(|| {
        Scrollable::vertical()
            .child_size(Constraints::loose(Vec2::new(f32::INFINITY, 240.0)))
            .show(|| {
                section(|| {
                    Pad::none().right(sizing::PADDING_XLARGE).show(|| {
                        col(|| {
                            let mut handle = |id: Id| {
                                radio(current_id, Some(id), || {
                                    draw(id);
                                })
                            };

                            if !text_buffer.is_empty() {
                                let mut filtered = ids
                                    .clone()
                                    .flat_map(|id| {
                                        let name = get_name(*id);
                                        let score = matcher.borrow().fuzzy_match(name, text_buffer);
                                        let score = score.unwrap_or(0).max(0) as usize;

                                        if score > (name.len() / 2) { Some((*id, score)) } else { None }
                                    })
                                    .collect::<Vec<_>>();

                                filtered.sort_by(|a, b| a.1.cmp(&b.1).reverse());

                                for (id, _) in filtered {
                                    handle(id);
                                }
                            } else {
                                ids.clone().for_each(|id| {
                                    handle(*id);
                                });
                            }
                        });
                    });
                });
            });
    });
}
