/// A "library" about general topics of the game like NPCs, Monsters, items,
/// skills etc.
pub(crate) struct Library {}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

impl Library {
    pub(crate) fn new() -> Self {
        Self {}
    }
}
