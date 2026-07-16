use std::fmt::{Display, Formatter};
use std::path::{Component, Path};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepoRelativePath(String);

impl RepoRelativePath {
    pub fn from_rooted(root: &Path, path: &Path) -> Result<Self, RepoPathError> {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| RepoPathError::OutsideRepository)?;
        let mut value = String::new();
        for component in relative.components() {
            let Component::Normal(segment) = component else {
                return Err(RepoPathError::NonNormalComponent);
            };
            let segment = segment.to_str().ok_or(RepoPathError::NonUtf8)?;
            if !value.is_empty() {
                value.push('/');
            }
            value.push_str(segment);
        }
        if value.is_empty() {
            return Err(RepoPathError::Empty);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Debug for RepoRelativePath {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("RepoRelativePath")
            .field(&self.0)
            .finish()
    }
}

impl Display for RepoRelativePath {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoPathError {
    OutsideRepository,
    NonNormalComponent,
    NonUtf8,
    Empty,
}
