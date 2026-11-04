//! Bind native composition to the field that owns the platform IME session.
use surface_app::{Event, ImeEvent};
use winit::event::Ime;

#[derive(Default)]
pub(crate) struct ImeSession {
    target: Option<String>,
    owner: Option<String>,
}

impl ImeSession {
    /// Restart the OS session when focus changes between editable fields.
    /// Some hosts only send `Enabled` once per OS session, not once per field.
    pub(crate) fn focus(&mut self, target: Option<String>) -> Vec<bool> {
        if self.target == target {
            return Vec::new();
        }
        self.owner = None;
        let mut allowed = Vec::with_capacity(2);
        if self.target.is_some() {
            allowed.push(false);
        }
        if target.is_some() {
            allowed.push(true);
        }
        self.target = target;
        allowed
    }

    pub(crate) fn event(&mut self, event: Ime) -> Option<Event> {
        let event = match event {
            Ime::Enabled => {
                self.owner.clone_from(&self.target);
                return None;
            }
            Ime::Disabled => {
                return self.owner.take().map(|id| Event::ImeFor {
                    id,
                    event: ImeEvent::Cancel,
                });
            }
            Ime::Preedit(text, selection) => ImeEvent::Preedit { text, selection },
            Ime::Commit(text) => ImeEvent::Commit(text),
        };
        self.owner.as_ref().map(|id| Event::ImeFor {
            id: id.clone(),
            event,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_switch_discards_old_composition_and_starts_a_new_session() {
        let mut session = ImeSession::default();
        assert_eq!(session.focus(Some("first".into())), [true]);
        session.event(Ime::Enabled);
        assert!(
            matches!(session.event(Ime::Preedit("ㄓ".into(), Some((3, 3)))),
            Some(Event::ImeFor { id, .. }) if id == "first")
        );

        assert_eq!(session.focus(Some("second".into())), [false, true]);
        assert!(session.event(Ime::Commit("中".into())).is_none());
        assert!(session.event(Ime::Disabled).is_none());
        session.event(Ime::Enabled);
        assert!(matches!(session.event(Ime::Commit("文".into())),
            Some(Event::ImeFor { id, event: ImeEvent::Commit(text) })
                if id == "second" && text == "文"));
    }

    #[test]
    fn blur_drops_late_commits_and_unchanged_focus_keeps_composition() {
        let mut session = ImeSession::default();
        session.focus(Some("name".into()));
        session.event(Ime::Enabled);
        assert!(session.focus(Some("name".into())).is_empty());
        assert!(session.event(Ime::Preedit("ㄨ".into(), None)).is_some());
        assert_eq!(session.focus(None), [false]);
        assert!(session.event(Ime::Commit("文".into())).is_none());
        session.event(Ime::Enabled);
        assert!(session.event(Ime::Commit("文".into())).is_none());
    }
}
