//! Immutable Package text objects and lazy composition recipes.
//!
//! This module is a process-local representation boundary. Exact Package bytes
//! retain owner-qualified custody and a cryptographic object identity; offsets
//! are meaningful only with the store generation that issued them. Recipes
//! preserve explicit separators and source spans, then flatten at a renderer or
//! other contiguous-byte boundary. None of these handles is a wire identity,
//! Package meaning, Genealogy payload, or native-owner truth.

use std::ops::Range;
use std::sync::{Arc, OnceLock};

use pale_core::records::{ArtifactDigest, PackageResourceRefV1};

/// An authored link, its declared target and its readable source location.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct ResourceTextLink {
    /// Whole Markdown link in the source document.
    pub(crate) range: Range<usize>,
    /// Destination bytes relative to the whole link.
    pub(crate) destination_range: Range<usize>,
    #[serde(flatten)]
    pub(crate) reference: ResourceTextReference,
}

/// A readable Package reference, independent of selection or authored syntax.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct ResourceTextReference {
    pub(crate) title: String,
    pub(crate) path: String,
    pub(crate) target: PackageResourceRefV1,
    pub(crate) fragment: Option<String>,
}

impl ResourceTextReference {
    pub(crate) fn location(&self) -> String {
        let fragment = self
            .fragment
            .as_ref()
            .map(|value| format!("#{value}"))
            .unwrap_or_default();
        format!(
            "{}, `{}`, `{}{fragment}`",
            self.title,
            self.target.package_id.as_str(),
            self.path
        )
    }
}

/// Domain separator for one deterministic Package text-store generation.
const TEXT_STORE_DOMAIN: &[u8] = b"pale.package-text-store.v1\0";

/// Exact immutable-byte identity of one stored text object.
#[derive(Clone, Debug, PartialEq, Eq)]
struct TextObjectId(ArtifactDigest);

/// One owner-qualified immutable Package text object.
#[derive(Debug)]
struct TextObject {
    id: TextObjectId,
    owner: PackageResourceRefV1,
    text: Arc<str>,
}

/// Generation-fenced store for one selected Package-text closure.
#[derive(Debug)]
struct TextStore {
    generation: ArtifactDigest,
    objects: Box<[TextObject]>,
}

/// Failure to resolve a process-local text reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextStoreError {
    /// The reference belongs to another immutable store generation.
    StaleGeneration,
    /// The object position or exact-byte identity does not match the store.
    WrongObject,
    /// The range is outside the object or splits a UTF-8 scalar value.
    InvalidRange,
}

/// One generation-local view over exact immutable Package bytes.
///
/// The tuple `(generation, object, object_id, range)` is mechanical custody,
/// not a durable semantic coordinate. The owner remains separately typed.
#[derive(Clone, Debug)]
pub(crate) struct TextRef {
    store: Arc<TextStore>,
    generation: ArtifactDigest,
    object: usize,
    object_id: TextObjectId,
    range: Range<usize>,
}

impl PartialEq for TextRef {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation
            && self.object == other.object
            && self.object_id == other.object_id
            && self.range == other.range
    }
}

impl Eq for TextRef {}

impl TextStore {
    /// Seal owner-qualified text objects in deterministic owner order.
    fn seal(
        entries: impl IntoIterator<Item = (PackageResourceRefV1, Arc<str>)>,
    ) -> Vec<(PackageResourceRefV1, TextRef)> {
        let mut entries = entries.into_iter().collect::<Vec<_>>();
        entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        let mut generation_preimage = Vec::new();
        generation_preimage.extend_from_slice(TEXT_STORE_DOMAIN);
        let objects = entries
            .into_iter()
            .map(|(owner, text)| {
                let id = TextObjectId(ArtifactDigest::hash_bytes(text.as_bytes()));
                append_framed(
                    &mut generation_preimage,
                    owner.package_id.as_str().as_bytes(),
                );
                append_framed(
                    &mut generation_preimage,
                    owner.resource_id.as_str().as_bytes(),
                );
                append_framed(&mut generation_preimage, id.0.as_str().as_bytes());
                TextObject { id, owner, text }
            })
            .collect::<Vec<_>>();
        let generation = ArtifactDigest::hash_bytes(&generation_preimage);
        let store = Arc::new(Self {
            generation: generation.clone(),
            objects: objects.into_boxed_slice(),
        });
        store
            .objects
            .iter()
            .enumerate()
            .map(|(object, entry)| {
                let reference = TextRef {
                    store: Arc::clone(&store),
                    generation: generation.clone(),
                    object,
                    object_id: entry.id.clone(),
                    range: 0..entry.text.len(),
                };
                (entry.owner.clone(), reference)
            })
            .collect()
    }

    fn resolve<'a>(&'a self, reference: &TextRef) -> Result<&'a str, TextStoreError> {
        if self.generation != reference.generation {
            return Err(TextStoreError::StaleGeneration);
        }
        let object = self
            .objects
            .get(reference.object)
            .filter(|object| object.id == reference.object_id)
            .ok_or(TextStoreError::WrongObject)?;
        object
            .text
            .get(reference.range.clone())
            .ok_or(TextStoreError::InvalidRange)
    }
}

impl TextRef {
    /// Borrow the exact referenced UTF-8 bytes.
    pub(crate) fn text(&self) -> &str {
        self.store
            .resolve(self)
            .expect("private references are validated before immutable store publication")
    }

    /// Create a generation-preserving UTF-8 subspan.
    pub(crate) fn slice(&self, range: Range<usize>) -> Option<Self> {
        if range.start > range.end {
            return None;
        }
        let start = self.range.start.checked_add(range.start)?;
        let end = self.range.start.checked_add(range.end)?;
        if end > self.range.end
            || self.store.objects[self.object]
                .text
                .get(start..end)
                .is_none()
        {
            return None;
        }
        Some(Self {
            store: Arc::clone(&self.store),
            generation: self.generation.clone(),
            object: self.object,
            object_id: self.object_id.clone(),
            range: start..end,
        })
    }

    /// Exact byte length of this span.
    pub(crate) fn len(&self) -> usize {
        self.range.len()
    }

    /// Whether this span contains no bytes.
    pub(crate) fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    /// Share the complete backing object when this reference spans it exactly.
    pub(crate) fn full_text(&self) -> Option<Arc<str>> {
        let object = self.store.objects.get(self.object)?;
        (self.range == (0..object.text.len())).then(|| Arc::clone(&object.text))
    }
}

/// One lazily flattened semantic-text composition.
///
/// Separators are explicit parts, so regrouping preserves the byte sequence by
/// the rendered associative law below. Materialization is memoized and shared
/// by every renderer that consumes the same recipe. There, `a`, `b`, and `c`
/// are recipes, `epsilon` is the empty recipe, `++` is recipe composition, and
/// the vertical operator concatenates their materialized bytes.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/text-recipe-composition.display.mathml"
))]
#[derive(Clone)]
pub(crate) struct TextRecipe(Arc<TextRecipeInner>);

struct TextRecipeInner {
    parts: Box<[TextPart]>,
    byte_len: usize,
    materialized: OnceLock<Arc<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TextPart {
    Slice(TextRef),
    Owned(Arc<str>),
    Static(&'static str),
    Hashes(u8),
    ResourceLink {
        source: TextRef,
        link: ResourceTextLink,
    },
}

impl std::fmt::Debug for TextRecipe {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TextRecipe")
            .field("parts", &self.0.parts.len())
            .field("bytes", &self.0.byte_len)
            .field("materialized", &self.0.materialized.get().is_some())
            .finish()
    }
}

impl PartialEq for TextRecipe {
    fn eq(&self, other: &Self) -> bool {
        self.0.parts == other.0.parts
    }
}

impl Eq for TextRecipe {}

impl TextRecipe {
    /// One generated owned part that carries no Package custody.
    pub(crate) fn owned(text: impl Into<Arc<str>>) -> Self {
        let mut builder = TextRecipeBuilder::default();
        builder.push_owned(text);
        builder.finish()
    }

    /// Return whether this recipe materializes no bytes.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.byte_len == 0
    }

    /// Materialize contiguous UTF-8 once at a consumer boundary.
    pub(crate) fn materialize(&self) -> Arc<String> {
        Arc::clone(self.0.materialized.get_or_init(|| {
            let mut text = String::with_capacity(self.0.byte_len);
            for part in &self.0.parts {
                match part {
                    TextPart::Slice(reference) => text.push_str(reference.text()),
                    TextPart::Owned(value) => text.push_str(value),
                    TextPart::Static(value) => text.push_str(value),
                    TextPart::Hashes(width) => {
                        text.extend(std::iter::repeat_n('#', usize::from(*width)));
                    }
                    TextPart::ResourceLink { source, .. } => text.push_str(source.text()),
                }
            }
            debug_assert_eq!(text.len(), self.0.byte_len);
            Arc::new(text)
        }))
    }

    /// Resolve installed links at the host/file boundary; name uninstalled targets.
    /// Exact source materialization and source custody remain unchanged.
    pub(crate) fn materialize_links<E>(
        &self,
        resolve: impl Fn(&PackageResourceRefV1, Option<&str>) -> Result<Option<String>, E>,
    ) -> Result<Arc<String>, E> {
        if !self
            .0
            .parts
            .iter()
            .any(|part| matches!(part, TextPart::ResourceLink { .. }))
        {
            return Ok(self.materialize());
        }
        let mut text = String::with_capacity(self.0.byte_len);
        for part in &self.0.parts {
            match part {
                TextPart::Slice(reference) => text.push_str(reference.text()),
                TextPart::Owned(value) => text.push_str(value),
                TextPart::Static(value) => text.push_str(value),
                TextPart::Hashes(width) => {
                    text.extend(std::iter::repeat_n('#', usize::from(*width)))
                }
                TextPart::ResourceLink { source, link } => {
                    if let Some(href) =
                        resolve(&link.reference.target, link.reference.fragment.as_deref())?
                    {
                        text.push_str(&source.text()[..link.destination_range.start]);
                        text.push_str(&href);
                        text.push_str(&source.text()[link.destination_range.end..]);
                    } else {
                        text.push_str(&link.reference.location());
                    }
                }
            }
        }
        Ok(Arc::new(text))
    }
}
impl From<String> for TextRecipe {
    fn from(value: String) -> Self {
        Self::owned(Arc::<str>::from(value))
    }
}
impl From<&'static str> for TextRecipe {
    fn from(value: &'static str) -> Self {
        let mut builder = TextRecipeBuilder::default();
        builder.push_static(value);
        builder.finish()
    }
}

/// Consuming builder for explicit text-recipe parts.
#[derive(Default)]
pub(crate) struct TextRecipeBuilder {
    parts: Vec<TextPart>,
    byte_len: usize,
}

impl TextRecipeBuilder {
    /// Whether no output-bearing parts have been appended.
    pub(crate) fn is_empty(&self) -> bool {
        self.byte_len == 0
    }

    /// Append an immutable source span without copying its bytes.
    pub(crate) fn push_ref(&mut self, reference: TextRef) {
        if reference.is_empty() {
            return;
        }
        self.byte_len += reference.len();
        self.parts.push(TextPart::Slice(reference));
    }

    /// Retain authored bytes and a qualified destination without selecting it.
    pub(crate) fn push_resource_link(&mut self, source: TextRef, link: &ResourceTextLink) {
        self.byte_len += source.len();
        self.parts.push(TextPart::ResourceLink {
            source,
            link: link.clone(),
        });
    }

    /// Append generated owned text without assigning it Package custody.
    pub(crate) fn push_owned(&mut self, text: impl Into<Arc<str>>) {
        let text = text.into();
        if text.is_empty() {
            return;
        }
        self.byte_len += text.len();
        self.parts.push(TextPart::Owned(text));
    }

    /// Append an exact compiler-owned separator or heading label.
    pub(crate) fn push_static(&mut self, text: &'static str) {
        if text.is_empty() {
            return;
        }
        self.byte_len += text.len();
        self.parts.push(TextPart::Static(text));
    }

    /// Append an admitted ATX marker width.
    pub(crate) fn push_hashes(&mut self, width: u8) {
        if width == 0 {
            return;
        }
        self.byte_len += usize::from(width);
        self.parts.push(TextPart::Hashes(width));
    }

    /// Append another recipe's exact parts without materializing it.
    pub(crate) fn append(&mut self, recipe: &TextRecipe) {
        self.byte_len += recipe.0.byte_len;
        self.parts.extend(recipe.0.parts.iter().cloned());
    }

    /// Seal this immutable recipe.
    pub(crate) fn finish(self) -> TextRecipe {
        TextRecipe(Arc::new(TextRecipeInner {
            parts: self.parts.into_boxed_slice(),
            byte_len: self.byte_len,
            materialized: OnceLock::new(),
        }))
    }
}

fn append_framed(target: &mut Vec<u8>, bytes: &[u8]) {
    let length = u64::try_from(bytes.len())
        .expect("a process-local text object length fits the portable digest frame");
    target.extend_from_slice(&length.to_be_bytes());
    target.extend_from_slice(bytes);
}

pub(crate) fn seal_texts(
    entries: impl IntoIterator<Item = (PackageResourceRefV1, Arc<str>)>,
) -> Vec<(PackageResourceRefV1, TextRef)> {
    TextStore::seal(entries)
}

#[cfg(test)]
#[path = "../../tests/internal/packages/text.rs"]
mod tests;
