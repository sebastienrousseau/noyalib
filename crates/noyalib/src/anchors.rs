//! Smart pointer anchor types for shared/DAG structures.
//!
//! These wrappers provide anchor semantics for `Rc` and `Arc` pointers,
//! allowing YAML serialization of shared data structures.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::ops::Deref;

#[cfg(not(feature = "std"))]
use alloc::rc::{Rc, Weak as RcWeak};
#[cfg(not(feature = "std"))]
use alloc::sync::Weak as ArcWeak;
#[cfg(feature = "std")]
use std::rc::{Rc, Weak as RcWeak};
#[cfg(feature = "std")]
use std::sync::Weak as ArcWeak;

use crate::prelude::FxHashMap;

/// Thread-local identity tracking for automatic anchor/alias emission.
///
/// Activated by `to_string_tracking_shared` (and writer variants). When active,
/// `RcAnchor`/`ArcAnchor`/`*WeakAnchor` consult this state during serialization:
/// the first time a pointer is seen, they emit a YAML anchor; subsequent
/// sightings emit an alias.
///
/// Not re-entrant across threads. `ArcAnchor` serialization remains on the
/// serialising thread; the scope guard ensures state does not leak across calls.
#[cfg(feature = "std")]
pub(crate) mod shared_tracking {
    use crate::prelude::FxHashMap;
    use core::cell::RefCell;

    pub(crate) enum TrackOutcome {
        NotActive,
        New(u32),
        Existing(u32),
    }

    struct AnchorState {
        seen: FxHashMap<usize, u32>,
        next_id: u32,
    }

    impl AnchorState {
        fn new() -> Self {
            Self {
                seen: FxHashMap::default(),
                next_id: 1,
            }
        }
    }

    std::thread_local! {
        static STATE: RefCell<Option<AnchorState>> = const { RefCell::new(None) };
    }

    /// RAII guard that installs a fresh tracking state on construction and
    /// clears it on drop. Nested scopes are rejected (only the outermost scope
    /// is authoritative) — this prevents accidental state bleed when users
    /// compose serializers.
    pub(crate) struct AnchorScope {
        owns: bool,
    }

    impl AnchorScope {
        pub(crate) fn enter() -> Self {
            let owns = STATE.with(|s| {
                let mut borrow = s.borrow_mut();
                if borrow.is_none() {
                    *borrow = Some(AnchorState::new());
                    true
                } else {
                    false
                }
            });
            Self { owns }
        }
    }

    impl Drop for AnchorScope {
        fn drop(&mut self) {
            if self.owns {
                STATE.with(|s| {
                    *s.borrow_mut() = None;
                });
            }
        }
    }

    /// Record a pointer; return whether it is newly seen or already tracked.
    pub(crate) fn track(ptr: usize) -> TrackOutcome {
        STATE.with(|s| {
            let mut borrow = s.borrow_mut();
            match borrow.as_mut() {
                None => TrackOutcome::NotActive,
                Some(state) => {
                    if let Some(&id) = state.seen.get(&ptr) {
                        TrackOutcome::Existing(id)
                    } else {
                        let id = state.next_id;
                        state.next_id = state.next_id.saturating_add(1);
                        let _ = state.seen.insert(ptr, id);
                        TrackOutcome::New(id)
                    }
                }
            }
        })
    }

    /// Look up without inserting. Used by weak-ref serializers: emit an alias
    /// only if the target was already anchored by a strong reference.
    pub(crate) fn peek(ptr: usize) -> Option<u32> {
        STATE.with(|s| {
            s.borrow()
                .as_ref()
                .and_then(|state| state.seen.get(&ptr).copied())
        })
    }

    pub(crate) fn format_id(id: u32) -> String {
        format!("id{id:03}")
    }
}

/// An `Rc` wrapper with YAML anchor semantics.
///
/// Serializes by delegating to the inner `T`. Deserializes by wrapping the
/// result in `Rc`.
///
/// # Examples
///
/// ```
/// use noyalib::RcAnchor;
/// let a: RcAnchor<String> = RcAnchor::from("shared".to_string());
/// assert_eq!(&*a, "shared");
/// ```
#[derive(Clone)]
pub struct RcAnchor<T>(pub Rc<T>);

impl<T: fmt::Debug> fmt::Debug for RcAnchor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RcAnchor").field(&self.0).finish()
    }
}

impl<T> Deref for RcAnchor<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> From<T> for RcAnchor<T> {
    fn from(v: T) -> Self {
        Self(Rc::new(v))
    }
}

impl<T> From<Rc<T>> for RcAnchor<T> {
    fn from(v: Rc<T>) -> Self {
        Self(v)
    }
}

impl<T> RcAnchor<T> {
    /// Unwrap into the inner `Rc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::RcAnchor;
    /// let a: RcAnchor<i32> = RcAnchor::from(7);
    /// let inner = a.into_inner();
    /// assert_eq!(*inner, 7);
    /// ```
    #[must_use]
    pub fn into_inner(self) -> Rc<T> {
        self.0
    }
}

impl<T: serde_core::Serialize> serde_core::Serialize for RcAnchor<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        #[cfg(feature = "std")]
        {
            let ptr = Rc::as_ptr(&self.0).cast::<()>() as usize;
            match shared_tracking::track(ptr) {
                shared_tracking::TrackOutcome::NotActive => self.0.serialize(serializer),
                shared_tracking::TrackOutcome::New(id) => {
                    let id_str = shared_tracking::format_id(id);
                    serializer
                        .serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_DEF, &(id_str, &*self.0))
                }
                shared_tracking::TrackOutcome::Existing(id) => {
                    let id_str = shared_tracking::format_id(id);
                    serializer.serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_REF, &id_str)
                }
            }
        }
        #[cfg(not(feature = "std"))]
        {
            self.0.serialize(serializer)
        }
    }
}

impl<'de, T: serde_core::Deserialize<'de>> serde_core::Deserialize<'de> for RcAnchor<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|v| Self(Rc::new(v)))
    }
}

/// An `Arc` wrapper with YAML anchor semantics.
///
/// Serializes by delegating to the inner `T`. Deserializes by wrapping the
/// result in `Arc`.
///
/// # Examples
///
/// ```
/// use noyalib::ArcAnchor;
/// let a: ArcAnchor<String> = ArcAnchor::from("shared".to_string());
/// assert_eq!(&*a, "shared");
/// ```
#[derive(Clone)]
pub struct ArcAnchor<T>(pub Arc<T>);

impl<T: fmt::Debug> fmt::Debug for ArcAnchor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ArcAnchor").field(&self.0).finish()
    }
}

impl<T> Deref for ArcAnchor<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> From<T> for ArcAnchor<T> {
    fn from(v: T) -> Self {
        Self(Arc::new(v))
    }
}

impl<T> From<Arc<T>> for ArcAnchor<T> {
    fn from(v: Arc<T>) -> Self {
        Self(v)
    }
}

impl<T> ArcAnchor<T> {
    /// Unwrap into the inner `Arc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ArcAnchor;
    /// let a: ArcAnchor<i32> = ArcAnchor::from(7);
    /// let inner = a.into_inner();
    /// assert_eq!(*inner, 7);
    /// ```
    #[must_use]
    pub fn into_inner(self) -> Arc<T> {
        self.0
    }
}

impl<T: serde_core::Serialize> serde_core::Serialize for ArcAnchor<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        #[cfg(feature = "std")]
        {
            let ptr = Arc::as_ptr(&self.0).cast::<()>() as usize;
            match shared_tracking::track(ptr) {
                shared_tracking::TrackOutcome::NotActive => self.0.serialize(serializer),
                shared_tracking::TrackOutcome::New(id) => {
                    let id_str = shared_tracking::format_id(id);
                    serializer
                        .serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_DEF, &(id_str, &*self.0))
                }
                shared_tracking::TrackOutcome::Existing(id) => {
                    let id_str = shared_tracking::format_id(id);
                    serializer.serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_REF, &id_str)
                }
            }
        }
        #[cfg(not(feature = "std"))]
        {
            self.0.serialize(serializer)
        }
    }
}

impl<'de, T: serde_core::Deserialize<'de>> serde_core::Deserialize<'de> for ArcAnchor<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|v| Self(Arc::new(v)))
    }
}

/// A weak `Rc` reference with YAML anchor semantics.
///
/// Serializes as `null` if the reference is dangling, otherwise serializes
/// the inner value. Deserialization from `null` produces a dangling weak ref.
///
/// # Examples
///
/// ```
/// use noyalib::RcWeakAnchor;
/// let w: RcWeakAnchor<i32> = RcWeakAnchor::dangling();
/// assert!(w.upgrade().is_none());
/// ```
#[derive(Clone)]
pub struct RcWeakAnchor<T>(pub RcWeak<T>);

impl<T: fmt::Debug> fmt::Debug for RcWeakAnchor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.upgrade() {
            Some(v) => f.debug_tuple("RcWeakAnchor").field(&v).finish(),
            None => f.debug_tuple("RcWeakAnchor").field(&"(dangling)").finish(),
        }
    }
}

impl<T> RcWeakAnchor<T> {
    /// Create a dangling weak anchor.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::RcWeakAnchor;
    /// let w: RcWeakAnchor<String> = RcWeakAnchor::dangling();
    /// assert!(w.upgrade().is_none());
    /// ```
    #[must_use]
    pub fn dangling() -> Self {
        Self(RcWeak::new())
    }

    /// Unwrap into the inner `Weak`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::RcWeakAnchor;
    /// let w: RcWeakAnchor<i32> = RcWeakAnchor::dangling();
    /// let inner = w.into_inner();
    /// assert!(inner.upgrade().is_none());
    /// ```
    #[must_use]
    pub fn into_inner(self) -> RcWeak<T> {
        self.0
    }

    /// Attempt to upgrade to a strong `Rc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::RcWeakAnchor;
    /// let w: RcWeakAnchor<i32> = RcWeakAnchor::dangling();
    /// assert!(w.upgrade().is_none());
    /// ```
    #[must_use]
    pub fn upgrade(&self) -> Option<Rc<T>> {
        self.0.upgrade()
    }
}

impl<T> From<RcWeak<T>> for RcWeakAnchor<T> {
    fn from(v: RcWeak<T>) -> Self {
        Self(v)
    }
}

impl<T: serde_core::Serialize> serde_core::Serialize for RcWeakAnchor<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        match self.0.upgrade() {
            Some(v) => {
                #[cfg(feature = "std")]
                {
                    // Weak refs never define a new anchor. If tracking is active
                    // and the target was already anchored by a strong reference,
                    // emit an alias; otherwise fall back to inline value.
                    let ptr = Rc::as_ptr(&v).cast::<()>() as usize;
                    if let Some(id) = shared_tracking::peek(ptr) {
                        let id_str = shared_tracking::format_id(id);
                        return serializer
                            .serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_REF, &id_str);
                    }
                }
                v.serialize(serializer)
            }
            None => serializer.serialize_none(),
        }
    }
}

impl<'de, T> serde_core::Deserialize<'de> for RcWeakAnchor<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        // Always deserialize as a dangling weak — there's no registry to look up.
        // We consume the value to avoid errors.
        let _ = serde_core::de::IgnoredAny::deserialize(deserializer)?;
        Ok(Self(RcWeak::new()))
    }
}

/// A weak `Arc` reference with YAML anchor semantics.
///
/// Serializes as `null` if the reference is dangling, otherwise serializes
/// the inner value. Deserialization from `null` produces a dangling weak ref.
///
/// # Examples
///
/// ```
/// use noyalib::ArcWeakAnchor;
/// let w: ArcWeakAnchor<i32> = ArcWeakAnchor::dangling();
/// assert!(w.upgrade().is_none());
/// ```
#[derive(Clone)]
pub struct ArcWeakAnchor<T>(pub ArcWeak<T>);

impl<T: fmt::Debug> fmt::Debug for ArcWeakAnchor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.upgrade() {
            Some(v) => f.debug_tuple("ArcWeakAnchor").field(&v).finish(),
            None => f.debug_tuple("ArcWeakAnchor").field(&"(dangling)").finish(),
        }
    }
}

impl<T> ArcWeakAnchor<T> {
    /// Create a dangling weak anchor.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ArcWeakAnchor;
    /// let w: ArcWeakAnchor<String> = ArcWeakAnchor::dangling();
    /// assert!(w.upgrade().is_none());
    /// ```
    #[must_use]
    pub fn dangling() -> Self {
        Self(ArcWeak::new())
    }

    /// Unwrap into the inner `Weak`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ArcWeakAnchor;
    /// let w: ArcWeakAnchor<i32> = ArcWeakAnchor::dangling();
    /// let inner = w.into_inner();
    /// assert!(inner.upgrade().is_none());
    /// ```
    #[must_use]
    pub fn into_inner(self) -> ArcWeak<T> {
        self.0
    }

    /// Attempt to upgrade to a strong `Arc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ArcWeakAnchor;
    /// let w: ArcWeakAnchor<i32> = ArcWeakAnchor::dangling();
    /// assert!(w.upgrade().is_none());
    /// ```
    #[must_use]
    pub fn upgrade(&self) -> Option<Arc<T>> {
        self.0.upgrade()
    }
}

impl<T> From<ArcWeak<T>> for ArcWeakAnchor<T> {
    fn from(v: ArcWeak<T>) -> Self {
        Self(v)
    }
}

impl<T: serde_core::Serialize> serde_core::Serialize for ArcWeakAnchor<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        match self.0.upgrade() {
            Some(v) => {
                #[cfg(feature = "std")]
                {
                    let ptr = Arc::as_ptr(&v).cast::<()>() as usize;
                    if let Some(id) = shared_tracking::peek(ptr) {
                        let id_str = shared_tracking::format_id(id);
                        return serializer
                            .serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_REF, &id_str);
                    }
                }
                v.serialize(serializer)
            }
            None => serializer.serialize_none(),
        }
    }
}

impl<'de, T> serde_core::Deserialize<'de> for ArcWeakAnchor<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        let _ = serde_core::de::IgnoredAny::deserialize(deserializer)?;
        Ok(Self(ArcWeak::new()))
    }
}

// ── Anchor Registries ──────────────────────────────────────────────────

/// Registry for shared `Rc` anchor references during deserialization.
///
/// When the same YAML anchor is referenced multiple times, all aliases
/// point to the same heap allocation via `Rc::clone`. This enables
/// true shared-memory DAG structures rather than duplicated subtrees.
///
/// # Examples
///
/// ```rust
/// use noyalib::AnchorRegistry;
/// use std::rc::Rc;
///
/// let mut reg = AnchorRegistry::<String>::new();
/// let rc = reg.register("shared".into(), "hello".into());
/// let alias = reg.resolve("shared").unwrap();
/// assert!(Rc::ptr_eq(&rc, &alias));
/// ```
pub struct AnchorRegistry<T> {
    anchors: FxHashMap<String, Rc<T>>,
}

impl<T: fmt::Debug> fmt::Debug for AnchorRegistry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnchorRegistry")
            .field("len", &self.anchors.len())
            .finish()
    }
}

impl<T> Default for AnchorRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AnchorRegistry<T> {
    /// Create an empty registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::AnchorRegistry;
    /// let reg = AnchorRegistry::<String>::new();
    /// assert!(reg.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            anchors: FxHashMap::default(),
        }
    }

    /// Register a value under the given anchor name.
    ///
    /// Returns the `Rc` wrapping the value. If an anchor with the
    /// same name already existed, the old entry is replaced.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::AnchorRegistry;
    /// let mut reg = AnchorRegistry::<i32>::new();
    /// let rc = reg.register("n".into(), 7);
    /// assert_eq!(*rc, 7);
    /// ```
    pub fn register(&mut self, name: String, value: T) -> Rc<T> {
        let rc = Rc::new(value);
        let _ = self.anchors.insert(name, Rc::clone(&rc));
        rc
    }

    /// Resolve an anchor by name, returning a cloned `Rc` if present.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::AnchorRegistry;
    /// let mut reg = AnchorRegistry::<i32>::new();
    /// let _ = reg.register("a".into(), 1);
    /// assert_eq!(*reg.resolve("a").unwrap(), 1);
    /// assert!(reg.resolve("missing").is_none());
    /// ```
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<Rc<T>> {
        self.anchors.get(name).cloned()
    }

    /// Returns the number of registered anchors.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::AnchorRegistry;
    /// let mut reg = AnchorRegistry::<i32>::new();
    /// let _ = reg.register("a".into(), 1);
    /// assert_eq!(reg.len(), 1);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// Returns `true` if no anchors are registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::AnchorRegistry;
    /// let reg = AnchorRegistry::<i32>::new();
    /// assert!(reg.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// Remove all entries from the registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::AnchorRegistry;
    /// let mut reg = AnchorRegistry::<i32>::new();
    /// let _ = reg.register("a".into(), 1);
    /// reg.clear();
    /// assert!(reg.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.anchors.clear();
    }
}

/// Registry for shared `Arc` anchor references during deserialization.
///
/// Thread-safe counterpart to [`AnchorRegistry`]. All aliases for the
/// same anchor share one `Arc` allocation, enabling cross-thread DAGs.
///
/// # Examples
///
/// ```rust
/// use noyalib::ArcAnchorRegistry;
/// use std::sync::Arc;
///
/// let mut reg = ArcAnchorRegistry::<String>::new();
/// let arc = reg.register("shared".into(), "hello".into());
/// let alias = reg.resolve("shared").unwrap();
/// assert!(Arc::ptr_eq(&arc, &alias));
/// ```
pub struct ArcAnchorRegistry<T> {
    anchors: FxHashMap<String, Arc<T>>,
}

impl<T: fmt::Debug> fmt::Debug for ArcAnchorRegistry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArcAnchorRegistry")
            .field("len", &self.anchors.len())
            .finish()
    }
}

impl<T> Default for ArcAnchorRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ArcAnchorRegistry<T> {
    /// Create an empty registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ArcAnchorRegistry;
    /// let reg = ArcAnchorRegistry::<String>::new();
    /// assert!(reg.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            anchors: FxHashMap::default(),
        }
    }

    /// Register a value under the given anchor name.
    ///
    /// Returns the `Arc` wrapping the value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ArcAnchorRegistry;
    /// let mut reg = ArcAnchorRegistry::<i32>::new();
    /// let arc = reg.register("n".into(), 7);
    /// assert_eq!(*arc, 7);
    /// ```
    pub fn register(&mut self, name: String, value: T) -> Arc<T> {
        let arc = Arc::new(value);
        let _ = self.anchors.insert(name, Arc::clone(&arc));
        arc
    }

    /// Resolve an anchor by name, returning a cloned `Arc` if present.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ArcAnchorRegistry;
    /// let mut reg = ArcAnchorRegistry::<i32>::new();
    /// let _ = reg.register("a".into(), 1);
    /// assert_eq!(*reg.resolve("a").unwrap(), 1);
    /// ```
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<Arc<T>> {
        self.anchors.get(name).cloned()
    }

    /// Returns the number of registered anchors.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ArcAnchorRegistry;
    /// let reg = ArcAnchorRegistry::<i32>::new();
    /// assert_eq!(reg.len(), 0);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// Returns `true` if no anchors are registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ArcAnchorRegistry;
    /// let reg = ArcAnchorRegistry::<i32>::new();
    /// assert!(reg.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// Remove all entries from the registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ArcAnchorRegistry;
    /// let mut reg = ArcAnchorRegistry::<i32>::new();
    /// let _ = reg.register("a".into(), 1);
    /// reg.clear();
    /// assert!(reg.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.anchors.clear();
    }
}

// ════════════════════════════════════════════════════════════════
// Issue #5 — recursive anchor types for cyclic YAML graphs.
// ════════════════════════════════════════════════════════════════

#[cfg(feature = "std")]
use std::cell::RefCell;
#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};

/// Single-threaded recursive anchor type for cyclic / late-initialised
/// YAML graphs.
///
/// Wraps `Rc<RefCell<Option<T>>>` so a value can be referenced by an
/// alias *before* the anchor is fully populated — the canonical
/// shape for self-referential YAML configs (call graphs, scene
/// trees, doubly-linked structures emitted as anchor + alias).
///
/// Access through [`RcRecursive::borrow`] / [`RcRecursive::borrow_mut`]
/// (not `Deref`) so the interior mutability is always explicit at
/// the call site — borrow-checker complaints surface in the YAML
/// code, not in the surrounding logic.
///
/// For thread-safe variants see [`ArcRecursive`].
///
/// # Examples
///
/// ```
/// use noyalib::RcRecursive;
/// let r: RcRecursive<String> = RcRecursive::empty();
/// assert!(r.borrow().is_none());
/// r.set("hello".to_string());
/// assert_eq!(r.borrow().as_deref(), Some("hello"));
/// ```
#[cfg(feature = "std")]
pub struct RcRecursive<T>(pub Rc<RefCell<Option<T>>>);

#[cfg(feature = "std")]
impl<T> Clone for RcRecursive<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[cfg(feature = "std")]
impl<T: fmt::Debug> fmt::Debug for RcRecursive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RcRecursive").field(&self.0).finish()
    }
}

#[cfg(feature = "std")]
impl<T> Default for RcRecursive<T> {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(feature = "std")]
impl<T> RcRecursive<T> {
    /// Construct an empty (uninitialised) recursive anchor.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::RcRecursive;
    /// let r: RcRecursive<i32> = RcRecursive::empty();
    /// assert!(r.borrow().is_none());
    /// ```
    #[must_use]
    pub fn empty() -> Self {
        Self(Rc::new(RefCell::new(None)))
    }

    /// Construct a recursive anchor pre-populated with `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::RcRecursive;
    /// let r = RcRecursive::new(7_i64);
    /// assert_eq!(r.borrow().as_ref().copied(), Some(7));
    /// ```
    #[must_use]
    pub fn new(value: T) -> Self {
        Self(Rc::new(RefCell::new(Some(value))))
    }

    /// Borrow the inner value immutably (runtime-checked).
    #[must_use]
    pub fn borrow(&self) -> core::cell::Ref<'_, Option<T>> {
        self.0.borrow()
    }

    /// Borrow the inner value mutably (runtime-checked).
    #[must_use]
    pub fn borrow_mut(&self) -> core::cell::RefMut<'_, Option<T>> {
        self.0.borrow_mut()
    }

    /// Replace the inner value, returning the previous one if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::RcRecursive;
    /// let r = RcRecursive::empty();
    /// assert!(r.set(1_i32).is_none());
    /// assert_eq!(r.set(2_i32), Some(1));
    /// ```
    pub fn set(&self, value: T) -> Option<T> {
        self.borrow_mut().replace(value)
    }

    /// Drop the inner value, returning it if any.
    #[must_use]
    pub fn take(&self) -> Option<T> {
        self.borrow_mut().take()
    }

    /// Number of strong `Rc` references to this recursive cell.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    /// Downgrade to an [`RcRecursion`] weak reference. Useful to
    /// break alias-only cycles when the anchored value is
    /// referenced from multiple places — the weak reference does
    /// not count towards the strong-count, so cycle storage is
    /// released as soon as the last strong [`RcRecursive`] drops.
    #[must_use]
    pub fn downgrade(&self) -> RcRecursion<T> {
        RcRecursion(Rc::downgrade(&self.0))
    }
}

#[cfg(feature = "std")]
impl<T: serde_core::Serialize> serde_core::Serialize for RcRecursive<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        match &*self.borrow() {
            Some(v) => v.serialize(serializer),
            None => serializer.serialize_unit(),
        }
    }
}

#[cfg(feature = "std")]
impl<'de, T: serde_core::Deserialize<'de>> serde_core::Deserialize<'de> for RcRecursive<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::new)
    }
}

/// Single-threaded weak recursive reference — pairs with
/// [`RcRecursive`].
///
/// Use to encode alias-only edges in a cyclic graph that should
/// not keep the anchored value alive on its own.
#[cfg(feature = "std")]
pub struct RcRecursion<T>(pub RcWeak<RefCell<Option<T>>>);

#[cfg(feature = "std")]
impl<T> Clone for RcRecursion<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[cfg(feature = "std")]
impl<T: fmt::Debug> fmt::Debug for RcRecursion<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RcRecursion").finish()
    }
}

#[cfg(feature = "std")]
impl<T> Default for RcRecursion<T> {
    fn default() -> Self {
        Self(RcWeak::new())
    }
}

#[cfg(feature = "std")]
impl<T> RcRecursion<T> {
    /// Attempt to upgrade to a strong [`RcRecursive`]. Returns
    /// `None` if every strong reference has been dropped.
    pub fn upgrade(&self) -> Option<RcRecursive<T>> {
        self.0.upgrade().map(RcRecursive)
    }
}

/// Thread-safe recursive anchor type — the [`RcRecursive`]
/// counterpart for cross-thread / parallel-parse use cases.
///
/// Wraps `Arc<Mutex<Option<T>>>`. Access through
/// [`ArcRecursive::lock`] (rather than a `Deref`) so the locking
/// is explicit at the call site.
///
/// # Examples
///
/// ```
/// use noyalib::ArcRecursive;
/// let r: ArcRecursive<i32> = ArcRecursive::new(42);
/// assert_eq!(*r.lock(), Some(42));
/// ```
#[cfg(feature = "std")]
pub struct ArcRecursive<T>(pub Arc<Mutex<Option<T>>>);

#[cfg(feature = "std")]
impl<T> Clone for ArcRecursive<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[cfg(feature = "std")]
impl<T: fmt::Debug> fmt::Debug for ArcRecursive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ArcRecursive").finish()
    }
}

#[cfg(feature = "std")]
impl<T> Default for ArcRecursive<T> {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(feature = "std")]
impl<T> ArcRecursive<T> {
    /// Construct an empty (uninitialised) thread-safe recursive
    /// anchor.
    #[must_use]
    pub fn empty() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }

    /// Construct a thread-safe recursive anchor pre-populated
    /// with `value`.
    #[must_use]
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(Some(value))))
    }

    /// Lock the inner cell. Recovers from poisoning rather than
    /// panicking — the only way the mutex gets poisoned is a
    /// panic mid-write inside the critical section, and the
    /// recovered guard is still observable as `None` or as the
    /// pre-panic value.
    ///
    /// Returns a `MutexGuard` over `Option<T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ArcRecursive;
    /// let r = ArcRecursive::new("hi".to_string());
    /// let guard = r.lock();
    /// assert_eq!(guard.as_deref(), Some("hi"));
    /// ```
    pub fn lock(&self) -> std::sync::MutexGuard<'_, Option<T>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Replace the inner value, returning the previous one if any.
    pub fn set(&self, value: T) -> Option<T> {
        self.lock().replace(value)
    }

    /// Drop the inner value, returning it if any.
    #[must_use]
    pub fn take(&self) -> Option<T> {
        self.lock().take()
    }

    /// Number of strong `Arc` references to this recursive cell.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    /// Downgrade to an [`ArcRecursion`] weak reference.
    #[must_use]
    pub fn downgrade(&self) -> ArcRecursion<T> {
        ArcRecursion(Arc::downgrade(&self.0))
    }
}

#[cfg(feature = "std")]
impl<T: serde_core::Serialize> serde_core::Serialize for ArcRecursive<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        match &*self.lock() {
            Some(v) => v.serialize(serializer),
            None => serializer.serialize_unit(),
        }
    }
}

#[cfg(feature = "std")]
impl<'de, T: serde_core::Deserialize<'de>> serde_core::Deserialize<'de> for ArcRecursive<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::new)
    }
}

/// Thread-safe weak recursive reference — pairs with
/// [`ArcRecursive`].
#[cfg(feature = "std")]
pub struct ArcRecursion<T>(pub ArcWeak<Mutex<Option<T>>>);

#[cfg(feature = "std")]
impl<T> Clone for ArcRecursion<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[cfg(feature = "std")]
impl<T: fmt::Debug> fmt::Debug for ArcRecursion<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ArcRecursion").finish()
    }
}

#[cfg(feature = "std")]
impl<T> Default for ArcRecursion<T> {
    fn default() -> Self {
        Self(ArcWeak::new())
    }
}

#[cfg(feature = "std")]
impl<T> ArcRecursion<T> {
    /// Attempt to upgrade to a strong [`ArcRecursive`]. Returns
    /// `None` if every strong reference has been dropped.
    pub fn upgrade(&self) -> Option<ArcRecursive<T>> {
        self.0.upgrade().map(ArcRecursive)
    }
}
