/// A stable dock identity in the reviewed ordered workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DockId {
    /// Scene hierarchy at upper left.
    Hierarchy,
    /// Scene/Game group in the upper workspace.
    Scene,
    /// Project/Console group below the workspace.
    Project,
    /// Full-height right Inspector.
    Inspector,
}
/// Stable tab identities; these do not identify scene objects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabId {
    /// Actual hierarchy panel.
    Hierarchy,
    /// Actual independent authoring preview.
    Scene,
    /// Explicit unsupported game-rendering diagnostic.
    Game,
    /// Actual read-only project browser.
    Project,
    /// Actual editor event console.
    Console,
    /// Actual active-document Inspector.
    Inspector,
}
impl TabId {
    pub(super) const ALL: [Self; 6] = [
        Self::Hierarchy,
        Self::Scene,
        Self::Game,
        Self::Project,
        Self::Console,
        Self::Inspector,
    ];
    pub(super) fn index(self) -> usize {
        match self {
            Self::Hierarchy => 0,
            Self::Scene => 1,
            Self::Game => 2,
            Self::Project => 3,
            Self::Console => 4,
            Self::Inspector => 5,
        }
    }
    /// Readable independently authored tab label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Hierarchy => "Hierarchy",
            Self::Scene => "Scene",
            Self::Game => "Game",
            Self::Project => "Project",
            Self::Console => "Console",
            Self::Inspector => "Inspector",
        }
    }
    /// Stable semantic role used for navigation and layout inspection.
    pub fn role(self) -> &'static str {
        match self {
            Self::Hierarchy => "tab.hierarchy",
            Self::Scene => "tab.scene",
            Self::Game => "tab.game",
            Self::Project => "tab.project",
            Self::Console => "tab.console",
            Self::Inspector => "tab.inspector",
        }
    }
}
/// The three adjacent-pair split boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SplitId {
    /// Left workspace versus full-height Inspector.
    Left,
    /// Upper workspace versus Project/Console.
    Upper,
    /// Hierarchy versus Scene/Game.
    Hierarchy,
}
