use balso_core::Bridge;

use super::Atom;

#[derive(Debug, PartialEq, Clone)]
pub enum Target {
    Atom(Atom),
    Bridge(Bridge),
}
